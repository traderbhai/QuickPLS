import {
  Box,
  ChevronDown,
  ChevronRight,
  Database,
  FileText,
  Folder,
  FolderOpen,
  FolderTree,
  Pencil,
  Plus,
  Save,
  Trash2,
  X,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type MouseEvent,
} from "react";
import type {
  NativeCanonicalModelSpec,
  NativeExplorerSelection,
  NativeSavedReport,
  RunMonitorStatus,
} from "../types";
import {
  formatNativeShortcut,
  nativeContextMenuCommands,
  type NativeCommandAction,
  type NativeCommandContext,
  type NativeSelectionKind,
  type ResolvedNativeCommand,
} from "./nativeCommands";
import { contextMenuCoordinates, nextEnabledItemIndex } from "./nativeMenuNavigation";
import {
  buildNativeWorkspaceTree,
  nextNativeWorkspaceModelName,
  nativeWorkspaceSelectionForNode,
  nativeWorkspaceTreeIdForSelection,
  nativeWorkspaceTreeNavigation,
  type NativeWorkspaceTreeNavigationKey,
  type NativeWorkspaceTreeNode,
} from "./nativeWorkspaceTree";

type ExplorerMutationResult = void | Promise<void>;

export interface NativeWorkspaceExplorerProps {
  projectName: string;
  projectPath: string | null;
  projectWritable: boolean;
  datasetName: string;
  datasetRows: number;
  datasetColumns: number;
  models: readonly NativeCanonicalModelSpec[];
  activeModelId: string | null;
  reports: readonly NativeSavedReport[];
  selection: NativeExplorerSelection;
  currentResultId: string | null;
  currentResultName?: string;
  currentResultSaved: boolean;
  calculationStatus: RunMonitorStatus;
  onSelectionChange: (selection: NativeExplorerSelection) => void;
  onOpenData: () => void;
  onOpenModel: (modelId: string) => void;
  onOpenReport: (resultId: string) => void;
  onCreateModel: (name: string) => ExplorerMutationResult;
  onRenameModel: (modelId: string, name: string) => ExplorerMutationResult;
  onDeleteModel: (modelId: string) => ExplorerMutationResult;
  onSaveReport: (resultId: string, name: string) => ExplorerMutationResult;
  onRenameReport: (resultId: string, name: string) => ExplorerMutationResult;
  onRemoveReport: (resultId: string) => ExplorerMutationResult;
}

export type NativeExplorerDialog =
  | { kind: "new-model"; initialValue: string }
  | { kind: "rename-model"; modelId: string; initialValue: string }
  | { kind: "delete-model"; modelId: string; name: string }
  | { kind: "save-report"; resultId: string; initialValue: string }
  | { kind: "rename-report"; resultId: string; initialValue: string }
  | { kind: "remove-report"; resultId: string; name: string };

interface ExplorerContextMenuState {
  x: number;
  y: number;
  nodeId: string;
  returnFocus: HTMLElement | null;
}

function explorerSelectionKind(node: NativeWorkspaceTreeNode): NativeSelectionKind {
  switch (node.kind) {
    case "data": return "project-data";
    case "models": return "project-models";
    case "model": return "project-model";
    case "reports": return "project-reports";
    case "report": return "project-report";
    case "project": return "none";
  }
}

export function NativeWorkspaceExplorer(props: NativeWorkspaceExplorerProps) {
  const {
    projectName,
    projectPath,
    projectWritable,
    datasetName,
    datasetRows,
    datasetColumns,
    models,
    activeModelId,
    reports,
    selection,
    currentResultId,
    currentResultName,
    currentResultSaved,
    calculationStatus,
    onSelectionChange,
    onOpenData,
    onOpenModel,
    onOpenReport,
    onCreateModel,
    onRenameModel,
    onDeleteModel,
    onSaveReport,
    onRenameReport,
    onRemoveReport,
  } = props;
  const [expandedIds, setExpandedIds] = useState<Set<string>>(() => new Set(["project", "models", "reports"]));
  const selectedTreeId = nativeWorkspaceTreeIdForSelection(selection);
  const [focusId, setFocusId] = useState(selectedTreeId);
  const [contextMenu, setContextMenu] = useState<ExplorerContextMenuState | null>(null);
  const [dialog, setDialog] = useState<NativeExplorerDialog | null>(null);
  const treeItemRefs = useRef(new Map<string, HTMLDivElement>());
  const dialogReturnFocus = useRef<HTMLElement | null>(null);

  const nodes = useMemo(() => buildNativeWorkspaceTree({
    projectName,
    datasetName,
    models,
    reports,
    expandedIds,
  }), [datasetName, expandedIds, models, projectName, reports]);

  useEffect(() => {
    if (nodes.some((node) => node.id === selectedTreeId)) setFocusId(selectedTreeId);
  }, [nodes, selectedTreeId]);

  useEffect(() => {
    if (nodes.some((node) => node.id === focusId)) return;
    const fallback = nodes.find((node) => node.id === selectedTreeId) ?? nodes[0];
    if (fallback) setFocusId(fallback.id);
  }, [focusId, nodes, selectedTreeId]);

  const setExpanded = useCallback((id: string, expanded: boolean) => {
    setExpandedIds((current) => {
      const next = new Set(current);
      if (expanded) next.add(id); else next.delete(id);
      return next;
    });
  }, []);

  const selectNode = useCallback((node: NativeWorkspaceTreeNode) => {
    setFocusId(node.id);
    const nextSelection = nativeWorkspaceSelectionForNode(node);
    if (nextSelection) onSelectionChange(nextSelection);
  }, [onSelectionChange]);

  const focusNode = useCallback((id: string) => {
    const node = nodes.find((candidate) => candidate.id === id);
    if (!node) return;
    selectNode(node);
    window.requestAnimationFrame(() => treeItemRefs.current.get(id)?.focus());
  }, [nodes, selectNode]);

  const openNode = useCallback((node: NativeWorkspaceTreeNode) => {
    selectNode(node);
    if (node.kind === "data") onOpenData();
    else if (node.kind === "model" && node.modelId) onOpenModel(node.modelId);
    else if (node.kind === "report" && node.resultId) onOpenReport(node.resultId);
    else if (node.expandable) setExpanded(node.id, !node.expanded);
  }, [onOpenData, onOpenModel, onOpenReport, selectNode, setExpanded]);

  const beginDialog = useCallback((next: NativeExplorerDialog, returnFocus?: HTMLElement | null) => {
    dialogReturnFocus.current = returnFocus ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
    setContextMenu(null);
    setDialog(next);
  }, []);

  const closeDialog = useCallback(() => {
    setDialog(null);
    window.setTimeout(() => {
      const target = dialogReturnFocus.current;
      if (target?.isConnected) target.focus();
      dialogReturnFocus.current = null;
    }, 0);
  }, []);

  const beginRename = useCallback((node: NativeWorkspaceTreeNode, returnFocus?: HTMLElement | null) => {
    if (node.kind === "model" && node.modelId) {
      beginDialog({ kind: "rename-model", modelId: node.modelId, initialValue: node.label }, returnFocus);
    } else if (node.kind === "report" && node.resultId) {
      beginDialog({ kind: "rename-report", resultId: node.resultId, initialValue: node.label }, returnFocus);
    }
  }, [beginDialog]);

  const beginDelete = useCallback((node: NativeWorkspaceTreeNode, returnFocus?: HTMLElement | null) => {
    if (node.kind === "model" && node.modelId) {
      beginDialog({ kind: "delete-model", modelId: node.modelId, name: node.label }, returnFocus);
    } else if (node.kind === "report" && node.resultId) {
      beginDialog({ kind: "remove-report", resultId: node.resultId, name: node.label }, returnFocus);
    }
  }, [beginDialog]);

  const contextForNode = useCallback((node: NativeWorkspaceTreeNode): NativeCommandContext => ({
    surface: "launcher",
    projectOpen: true,
    projectWritable,
    hasDataset: datasetColumns > 0,
    hasCompletedRun: Boolean(currentResultId),
    selectedResultSaved: currentResultSaved,
    canOpenContextModel: Boolean(activeModelId && models.some((model) => model.id === activeModelId)),
    canCalculate: false,
    canUndo: false,
    canRedo: false,
    canRecode: false,
    canConfigureGroups: false,
    selectedVariableIsGrouping: false,
    canAddModeration: false,
    canAddHigherOrder: false,
    propertiesOpen: false,
    selection: { kind: explorerSelectionKind(node), count: node.kind === "project" ? 0 : 1 },
    calculationStatus,
  }), [activeModelId, calculationStatus, currentResultId, currentResultSaved, datasetColumns, models, projectWritable]);

  const contextCommands = useMemo(() => {
    if (!contextMenu) return [];
    const node = nodes.find((candidate) => candidate.id === contextMenu.nodeId);
    return node ? nativeContextMenuCommands(contextForNode(node)) : [];
  }, [contextForNode, contextMenu, nodes]);

  const executeAction = useCallback((action: NativeCommandAction, node?: NativeWorkspaceTreeNode, returnFocus?: HTMLElement | null) => {
    const target = node ?? nodes.find((candidate) => candidate.id === selectedTreeId);
    switch (action.id) {
      case "explorer.open-selection":
        if (target) openNode(target);
        return;
      case "explorer.new-model":
        beginDialog({ kind: "new-model", initialValue: nextNativeWorkspaceModelName(models) }, returnFocus);
        return;
      case "explorer.rename-selection":
        if (target) beginRename(target, returnFocus);
        return;
      case "explorer.delete-selection":
        if (target) beginDelete(target, returnFocus);
        return;
      case "explorer.save-report":
        if (currentResultId && !currentResultSaved) beginDialog({
          kind: "save-report",
          resultId: currentResultId,
          initialValue: currentResultName?.trim() || "Results report",
        }, returnFocus);
        return;
      default:
        return;
    }
  }, [beginDelete, beginDialog, beginRename, currentResultId, currentResultName, currentResultSaved, models, nodes, openNode, selectedTreeId]);

  useEffect(() => {
    if (!contextMenu) return;
    const closeFromPointer = (event: PointerEvent) => {
      if (!(event.target instanceof Element) || !event.target.closest(".nd-explorer-context-menu")) setContextMenu(null);
    };
    document.addEventListener("pointerdown", closeFromPointer);
    return () => document.removeEventListener("pointerdown", closeFromPointer);
  }, [contextMenu]);

  const showContextMenu = useCallback((node: NativeWorkspaceTreeNode, x: number, y: number, returnFocus: HTMLElement) => {
    const commands = nativeContextMenuCommands(contextForNode(node));
    if (!commands.length) return;
    selectNode(node);
    const position = contextMenuCoordinates(x, y, window.innerWidth, window.innerHeight, 230, 12 + commands.length * 29);
    setContextMenu({ ...position, nodeId: node.id, returnFocus });
  }, [contextForNode, selectNode]);

  const openPointerContextMenu = useCallback((event: MouseEvent<HTMLElement>, node: NativeWorkspaceTreeNode) => {
    event.preventDefault();
    event.stopPropagation();
    showContextMenu(node, event.clientX, event.clientY, event.currentTarget);
  }, [showContextMenu]);

  const onTreeItemKeyDown = (event: KeyboardEvent<HTMLDivElement>, node: NativeWorkspaceTreeNode) => {
    if (event.key === "Enter") {
      event.preventDefault();
      event.stopPropagation();
      openNode(node);
      return;
    }
    if (event.key === "F2" && ["model", "report"].includes(node.kind)) {
      event.preventDefault();
      event.stopPropagation();
      const rename = nativeContextMenuCommands(contextForNode(node))
        .find((command) => command.action.id === "explorer.rename-selection");
      if (rename?.enabled) beginRename(node, event.currentTarget);
      return;
    }
    if (event.key === "Delete" && ["model", "report"].includes(node.kind)) {
      event.preventDefault();
      event.stopPropagation();
      const remove = nativeContextMenuCommands(contextForNode(node))
        .find((command) => command.action.id === "explorer.delete-selection");
      if (remove?.enabled) beginDelete(node, event.currentTarget);
      return;
    }
    if (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey)) {
      event.preventDefault();
      event.stopPropagation();
      const bounds = event.currentTarget.getBoundingClientRect();
      showContextMenu(node, bounds.left + Math.min(24, Math.max(8, bounds.width / 2)), bounds.bottom, event.currentTarget);
      return;
    }
    if (!["ArrowDown", "ArrowUp", "ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key)) return;
    const navigation = nativeWorkspaceTreeNavigation(nodes, node.id, event.key as NativeWorkspaceTreeNavigationKey);
    if (!navigation) return;
    event.preventDefault();
    event.stopPropagation();
    if (navigation.expansion) setExpanded(navigation.expansion.id, navigation.expansion.expanded);
    focusNode(navigation.focusId);
  };

  const selectedNode = nodes.find((node) => node.id === selectedTreeId) ?? nodes.find((node) => node.id === focusId) ?? nodes[0];
  const selectedModel = selection.kind === "model" ? models.find((model) => model.id === selection.modelId) : undefined;
  const selectedReport = selection.kind === "report" ? reports.find((report) => report.resultId === selection.resultId) : undefined;
  const mutationLocked = !projectWritable || ["queued", "validating", "running", "cancelling"].includes(calculationStatus);

  return <div className="nd-project-workspace" aria-label="Project workspace">
    <aside className="nd-workspace-explorer" aria-labelledby="nd-workspace-explorer-title">
      <header className="nd-pane-title nd-explorer-titlebar">
        <FolderTree size={14} aria-hidden="true" />
        <strong id="nd-workspace-explorer-title">Project Explorer</strong>
      </header>
      <div className="nd-project-tree" role="tree" aria-label={`${projectName} project contents`}>
        {nodes.map((node) => {
          const NodeIcon = node.kind === "project"
            ? node.expanded ? FolderOpen : Folder
            : node.kind === "data"
              ? Database
              : node.kind === "model"
                ? Box
                : node.kind === "report"
                  ? FileText
                  : node.expanded ? FolderOpen : Folder;
          const selected = selectedTreeId === node.id;
          return <div
            key={node.id}
            ref={(element) => { if (element) treeItemRefs.current.set(node.id, element); else treeItemRefs.current.delete(node.id); }}
            className={`nd-project-treeitem${selected ? " selected" : ""}${node.modelId === activeModelId ? " active-model" : ""}`}
            role="treeitem"
            aria-level={node.level}
            aria-selected={selected}
            aria-expanded={node.expandable ? node.expanded : undefined}
            tabIndex={focusId === node.id ? 0 : -1}
            data-kind={node.kind}
            style={{ paddingLeft: 6 + (node.level - 1) * 16 } as CSSProperties}
            onClick={() => selectNode(node)}
            onDoubleClick={() => openNode(node)}
            onContextMenu={(event) => openPointerContextMenu(event, node)}
            onKeyDown={(event) => onTreeItemKeyDown(event, node)}
          >
            <span className="nd-tree-expander" aria-hidden="true">{node.expandable ? node.expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} /> : null}</span>
            <NodeIcon size={14} aria-hidden="true" />
            <span className="nd-tree-label">{node.label}</span>
            {node.modelId === activeModelId ? <span className="nd-active-model-mark" title="Active model" aria-label="Active model" /> : null}
          </div>;
        })}
      </div>
    </aside>
    <section className="nd-explorer-detail" aria-labelledby="nd-explorer-detail-title">
      <ExplorerDetail
        selectedNode={selectedNode}
        projectName={projectName}
        projectPath={projectPath}
        projectWritable={projectWritable}
        datasetName={datasetName}
        datasetRows={datasetRows}
        datasetColumns={datasetColumns}
        modelCount={models.length}
        reportCount={reports.length}
        activeModelId={activeModelId}
        activeModelName={models.find((model) => model.id === activeModelId)?.name ?? null}
        selectedModel={selectedModel}
        selectedReport={selectedReport}
        currentResultId={currentResultId}
        currentResultSaved={currentResultSaved}
        mutationLocked={mutationLocked}
        open={() => selectedNode && openNode(selectedNode)}
        createModel={(element) => beginDialog({ kind: "new-model", initialValue: nextNativeWorkspaceModelName(models) }, element)}
        rename={(element) => selectedNode && beginRename(selectedNode, element)}
        remove={(element) => selectedNode && beginDelete(selectedNode, element)}
        saveReport={(element) => currentResultId && beginDialog({ kind: "save-report", resultId: currentResultId, initialValue: currentResultName?.trim() || "Results report" }, element)}
      />
    </section>
    {contextMenu && selectedNode ? <ExplorerContextMenu
      commands={contextCommands}
      state={contextMenu}
      close={(restoreFocus) => {
        setContextMenu(null);
        if (restoreFocus && contextMenu.returnFocus?.isConnected) contextMenu.returnFocus.focus();
      }}
      execute={(command) => {
        const target = nodes.find((node) => node.id === contextMenu.nodeId);
        setContextMenu(null);
        executeAction(command.action, target, contextMenu.returnFocus);
      }}
    /> : null}
    {dialog ? <NativeWorkspaceExplorerDialog
      dialog={dialog}
      close={closeDialog}
      onCreateModel={onCreateModel}
      onRenameModel={onRenameModel}
      onDeleteModel={onDeleteModel}
      onSaveReport={onSaveReport}
      onRenameReport={onRenameReport}
      onRemoveReport={onRemoveReport}
    /> : null}
  </div>;
}

function ExplorerDetail({
  selectedNode,
  projectName,
  projectPath,
  projectWritable,
  datasetName,
  datasetRows,
  datasetColumns,
  modelCount,
  reportCount,
  activeModelId,
  activeModelName,
  selectedModel,
  selectedReport,
  currentResultId,
  currentResultSaved,
  mutationLocked,
  open,
  createModel,
  rename,
  remove,
  saveReport,
}: {
  selectedNode?: NativeWorkspaceTreeNode;
  projectName: string;
  projectPath: string | null;
  projectWritable: boolean;
  datasetName: string;
  datasetRows: number;
  datasetColumns: number;
  modelCount: number;
  reportCount: number;
  activeModelId: string | null;
  activeModelName: string | null;
  selectedModel?: NativeCanonicalModelSpec;
  selectedReport?: NativeSavedReport;
  currentResultId: string | null;
  currentResultSaved: boolean;
  mutationLocked: boolean;
  open: () => void;
  createModel: (element: HTMLElement) => void;
  rename: (element: HTMLElement) => void;
  remove: (element: HTMLElement) => void;
  saveReport: (element: HTMLElement) => void;
}) {
  const kind = selectedNode?.kind ?? "project";
  const heading = selectedNode?.label ?? projectName;
  const canOpen = ["data", "model", "report"].includes(kind);
  const canRename = ["model", "report"].includes(kind);
  const isDestructive = ["model", "report"].includes(kind);
  return <>
    <header className="nd-explorer-detail-header">
      <h1 id="nd-explorer-detail-title">{heading}</h1>
      {kind === "model" && selectedModel?.id === activeModelId ? <span className="nd-detail-state">Active model</span> : null}
      {!projectWritable ? <span className="nd-detail-state">Read-only</span> : null}
    </header>
    <dl className="nd-explorer-properties">
      {kind === "project" ? <>
        <div><dt>Project</dt><dd>{projectName}</dd></div>
        <div><dt>Location</dt><dd title={projectPath ?? "Not saved"}>{projectPath ?? "Not saved"}</dd></div>
        <div><dt>Data</dt><dd>{datasetName}</dd></div>
        <div><dt>Models</dt><dd>{modelCount}</dd></div>
        <div><dt>Reports</dt><dd>{reportCount}</dd></div>
      </> : null}
      {kind === "data" ? <>
        <div><dt>Dataset</dt><dd>{datasetName}</dd></div>
        <div><dt>Cases</dt><dd>{datasetRows.toLocaleString()}</dd></div>
        <div><dt>Variables</dt><dd>{datasetColumns.toLocaleString()}</dd></div>
      </> : null}
      {kind === "models" ? <>
        <div><dt>Models</dt><dd>{modelCount}</dd></div>
        <div><dt>Active model</dt><dd>{activeModelName ?? "None"}</dd></div>
      </> : null}
      {kind === "model" && selectedModel ? <>
        <div><dt>Name</dt><dd>{selectedModel.name}</dd></div>
        <div><dt>Constructs</dt><dd>{selectedModel.constructs.length}</dd></div>
        <div><dt>Relationships</dt><dd>{selectedModel.paths.length + selectedModel.controls.length}</dd></div>
      </> : null}
      {kind === "reports" ? <>
        <div><dt>Saved reports</dt><dd>{reportCount}</dd></div>
        <div><dt>Current result</dt><dd>{currentResultId ? currentResultSaved ? "Saved" : "Available" : "None"}</dd></div>
      </> : null}
      {kind === "report" && selectedReport ? <>
        <div><dt>Name</dt><dd>{selectedReport.name}</dd></div>
        <div><dt>Saved</dt><dd>{formatSavedAt(selectedReport.savedAt)}</dd></div>
        <div><dt>Result</dt><dd title={selectedReport.resultId}>{selectedReport.resultId}</dd></div>
      </> : null}
    </dl>
    <footer className="nd-explorer-detail-actions">
      {canOpen ? <button type="button" className="primary" onClick={open}>Open</button> : null}
      {kind === "models" ? <button type="button" disabled={mutationLocked} onClick={(event) => createModel(event.currentTarget)}><Plus size={14} /> New Model</button> : null}
      {kind === "reports" && !currentResultSaved ? <button type="button" disabled={mutationLocked || !currentResultId} onClick={(event) => saveReport(event.currentTarget)}><Save size={14} /> Save Report</button> : null}
      {canRename ? <button type="button" disabled={mutationLocked} onClick={(event) => rename(event.currentTarget)}><Pencil size={14} /> Rename</button> : null}
      {isDestructive ? <button type="button" className="danger" disabled={mutationLocked} onClick={(event) => remove(event.currentTarget)}><Trash2 size={14} /> {kind === "model" ? "Delete" : "Remove"}</button> : null}
    </footer>
  </>;
}

function ExplorerContextMenu({ commands, state, close, execute }: {
  commands: readonly ResolvedNativeCommand[];
  state: ExplorerContextMenuState;
  close: (restoreFocus: boolean) => void;
  execute: (command: ResolvedNativeCommand) => void;
}) {
  const menuRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    menuRef.current?.querySelector<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')?.focus();
  }, []);
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close(true);
      return;
    }
    if (event.key === "Tab") {
      close(false);
      return;
    }
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const buttons = Array.from(menuRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? []);
    const next = nextEnabledItemIndex(buttons.map((button) => button.disabled), buttons.indexOf(document.activeElement as HTMLButtonElement), event.key as "ArrowDown" | "ArrowUp" | "Home" | "End");
    if (next >= 0) buttons[next]?.focus();
  };
  return <div
    ref={menuRef}
    className="nd-context-menu nd-explorer-context-menu"
    role="menu"
    aria-label="Project item commands"
    style={{ left: state.x, top: state.y }}
    onKeyDown={onKeyDown}
  >
    {commands.map((command) => <button
      key={command.id}
      type="button"
      role="menuitem"
      tabIndex={-1}
      className={command.contextMenu?.some((placement) => placement.surface === "launcher" && placement.separatorBefore) ? "separator" : ""}
      disabled={!command.enabled}
      onClick={() => execute(command)}
    ><span>{command.label}</span>{command.shortcut ? <kbd>{formatNativeShortcut(command.shortcut)}</kbd> : null}</button>)}
  </div>;
}

export function NativeWorkspaceExplorerDialog({
  dialog,
  close,
  onCreateModel,
  onRenameModel,
  onDeleteModel,
  onSaveReport,
  onRenameReport,
  onRemoveReport,
}: {
  dialog: NativeExplorerDialog;
  close: () => void;
  onCreateModel: (name: string) => ExplorerMutationResult;
  onRenameModel: (modelId: string, name: string) => ExplorerMutationResult;
  onDeleteModel: (modelId: string) => ExplorerMutationResult;
  onSaveReport: (resultId: string, name: string) => ExplorerMutationResult;
  onRenameReport: (resultId: string, name: string) => ExplorerMutationResult;
  onRemoveReport: (resultId: string) => ExplorerMutationResult;
}) {
  const rootRef = useRef<HTMLDivElement>(null);
  const [value, setValue] = useState("initialValue" in dialog ? dialog.initialValue : "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const destructive = dialog.kind === "delete-model" || dialog.kind === "remove-report";
  const title = dialog.kind === "new-model"
    ? "New Model"
    : dialog.kind === "rename-model"
      ? "Rename Model"
      : dialog.kind === "delete-model"
        ? "Delete Model"
        : dialog.kind === "save-report"
          ? "Save Report"
          : dialog.kind === "rename-report"
            ? "Rename Report"
            : "Remove Report";
  const actionLabel = dialog.kind === "new-model"
    ? "Create"
    : dialog.kind === "save-report"
      ? "Save"
      : dialog.kind === "delete-model"
        ? "Delete"
        : dialog.kind === "remove-report"
          ? "Remove"
          : "Rename";

  useLayoutEffect(() => {
    const root = rootRef.current;
    const input = root?.querySelector<HTMLInputElement>("input");
    const target = input ?? root?.querySelector<HTMLButtonElement>("button.primary, button.danger");
    target?.focus();
    input?.select();
  }, []);

  const submit = async () => {
    const name = value.trim();
    if (!destructive && !name) {
      setError("Enter a name.");
      rootRef.current?.querySelector<HTMLInputElement>("input")?.focus();
      return;
    }
    setBusy(true);
    setError("");
    try {
      if (dialog.kind === "new-model") await onCreateModel(name);
      else if (dialog.kind === "rename-model") await onRenameModel(dialog.modelId, name);
      else if (dialog.kind === "delete-model") await onDeleteModel(dialog.modelId);
      else if (dialog.kind === "save-report") await onSaveReport(dialog.resultId, name);
      else if (dialog.kind === "rename-report") await onRenameReport(dialog.resultId, name);
      else await onRemoveReport(dialog.resultId);
      close();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "The project item could not be updated.");
      window.setTimeout(() => {
        const root = rootRef.current;
        (root?.querySelector<HTMLInputElement>("input")
          ?? root?.querySelector<HTMLButtonElement>("button.danger, button.primary"))?.focus();
      }, 0);
    } finally {
      setBusy(false);
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === "Escape" && !busy) {
      event.preventDefault();
      event.stopPropagation();
      close();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = Array.from(rootRef.current?.querySelectorAll<HTMLElement>('input, button:not(:disabled), [tabindex]:not([tabindex="-1"])') ?? []);
    if (!focusable.length) return;
    const index = focusable.indexOf(document.activeElement as HTMLElement);
    const next = event.shiftKey
      ? index <= 0 ? focusable.length - 1 : index - 1
      : index === focusable.length - 1 ? 0 : index + 1;
    event.preventDefault();
    focusable[next]?.focus();
  };

  return <div className="nd-dialog-backdrop nd-explorer-dialog-backdrop" role="presentation" onMouseDown={(event) => { if (!busy && event.target === event.currentTarget) close(); }}>
    <div ref={rootRef} className="nd-dialog nd-dialog-explorer" role="dialog" aria-modal="true" aria-labelledby="nd-explorer-dialog-title" aria-describedby={destructive ? "nd-explorer-dialog-description" : undefined} onKeyDown={onKeyDown}>
      <header><h2 id="nd-explorer-dialog-title">{title}</h2>{!busy ? <button type="button" aria-label="Close dialog" onClick={close}><X size={15} /></button> : null}</header>
      <form className="nd-dialog-form nd-explorer-dialog-form" onSubmit={(event) => { event.preventDefault(); void submit(); }}>
        {destructive ? <p id="nd-explorer-dialog-description">{dialog.kind === "delete-model"
          ? <>Delete <strong>{dialog.name}</strong>? Its model layout will be removed from this project. Completed results remain in run history.</>
          : <>Remove <strong>{dialog.name}</strong> from Saved Reports? The completed result remains in run history.</>}</p> : <label>
          <span>Name</span>
          <input value={value} maxLength={120} onChange={(event) => { setValue(event.target.value); if (error) setError(""); }} aria-invalid={Boolean(error)} aria-describedby={error ? "nd-explorer-dialog-error" : undefined} />
        </label>}
        {error ? <p id="nd-explorer-dialog-error" className="nd-form-error" role="alert">{error}</p> : null}
        <footer>
          <button type="button" disabled={busy} onClick={close}>Cancel</button>
          <button type="submit" className={destructive ? "danger" : "primary"} disabled={busy || (!destructive && !value.trim())}>{busy ? "Working..." : actionLabel}</button>
        </footer>
      </form>
    </div>
  </div>;
}

function formatSavedAt(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.valueOf()) ? "Saved" : date.toLocaleString();
}

export default NativeWorkspaceExplorer;
