import type { RunMonitorStatus } from "../types";

/** Primary documents in the native desktop shell. */
export type NativeSurface = "launcher" | "data" | "model" | "results";

export type NativeSelectionKind =
  | "none"
  | "dataset"
  | "variable"
  | "construct"
  | "path"
  | "result"
  | "project-data"
  | "project-models"
  | "project-model"
  | "project-reports"
  | "project-report"
  | "multiple";

export type NativeMenuId = "file" | "edit" | "view" | "calculate" | "tools" | "help";

export interface NativeCommandContext {
  surface: NativeSurface;
  projectOpen: boolean;
  projectWritable: boolean;
  hasDataset: boolean;
  hasCompletedRun: boolean;
  selectedResultSaved: boolean;
  canOpenContextModel: boolean;
  canCalculate: boolean;
  canUndo: boolean;
  canRedo: boolean;
  canRecode: boolean;
  canConfigureGroups: boolean;
  selectedVariableIsGrouping: boolean;
  canAddModeration: boolean;
  canAddHigherOrder: boolean;
  propertiesOpen: boolean;
  selection: {
    kind: NativeSelectionKind;
    count: number;
  };
  calculationStatus: RunMonitorStatus;
}

export interface NativeKeyboardShortcut {
  key: string;
  ctrl?: boolean;
  shift?: boolean;
  alt?: boolean;
  meta?: boolean;
  /** Global application shortcuts may opt in; single-key model tools never do. */
  allowInEditable?: boolean;
}

export type NativeCommandAction =
  | { id: "project.new" }
  | { id: "project.open" }
  | { id: "project.open-demo" }
  | { id: "project.import-data" }
  | { id: "data.recode" }
  | { id: "data.configure-groups" }
  | { id: "project.save"; saveAs?: boolean }
  | { id: "explorer.open-selection" }
  | { id: "explorer.new-model" }
  | { id: "explorer.rename-selection" }
  | { id: "explorer.delete-selection" }
  | { id: "explorer.save-report" }
  | { id: "surface.navigate"; surface: NativeSurface }
  | { id: "model.undo" }
  | { id: "model.redo" }
  | { id: "model.set-tool"; tool: "select" | "pan" | "path" }
  | { id: "model.add-construct" }
  | { id: "model.add-higher-order" }
  | { id: "model.add-moderating-effect" }
  | { id: "model.edit-selection" }
  | { id: "model.delete-selection" }
  | { id: "model.arrange"; strategy: "smartpls" }
  | { id: "model.fit" }
  | { id: "calculation.open" }
  | { id: "calculation.cancel" }
  | { id: "results.export" }
  | { id: "results.open-run-details" }
  | { id: "view.toggle-properties" }
  | { id: "utility.open"; utility: "method-scope" | "preferences" | "shortcuts" | "about" };

export type NativeActionId = NativeCommandAction["id"];

export interface NativeMenuPlacement {
  menu: NativeMenuId;
  order: number;
  separatorBefore?: boolean;
}

export interface NativeToolbarPlacement {
  surface: NativeSurface;
  order: number;
  primary?: boolean;
}

export interface NativeContextPlacement {
  surface: NativeSurface;
  order: number;
  selections?: readonly NativeSelectionKind[];
  separatorBefore?: boolean;
}

type ContextRule = (context: Readonly<NativeCommandContext>) => boolean;
type CommandLabel = string | ((context: Readonly<NativeCommandContext>) => string);

export interface NativeCommandDefinition {
  id: NativeCommandId;
  label: CommandLabel;
  action: NativeCommandAction;
  shortcut?: NativeKeyboardShortcut;
  menu?: NativeMenuPlacement;
  toolbar?: readonly NativeToolbarPlacement[];
  contextMenu?: readonly NativeContextPlacement[];
  visibleWhen?: ContextRule;
  enabledWhen?: ContextRule;
}

export type NativeCommandId =
  | "new-project"
  | "open-project"
  | "open-demo-project"
  | "import-data"
  | "recode-variable"
  | "configure-groups"
  | "save-project"
  | "save-project-as"
  | "open-project-item"
  | "new-model"
  | "save-report"
  | "rename-project-item"
  | "delete-project-item"
  | "undo"
  | "redo"
  | "go-project"
  | "go-data"
  | "go-model"
  | "go-result-data"
  | "go-results"
  | "toggle-properties"
  | "select-tool"
  | "pan-tool"
  | "add-construct"
  | "path-tool"
  | "add-higher-order"
  | "add-moderating-effect"
  | "edit-selection"
  | "delete-selection"
  | "arrange-model"
  | "fit-model"
  | "open-calculation"
  | "cancel-calculation"
  | "export-results"
  | "run-details"
  | "method-scope"
  | "preferences"
  | "keyboard-shortcuts"
  | "about";

const ACTIVE_CALCULATION_STATES: ReadonlySet<RunMonitorStatus> = new Set([
  "queued",
  "validating",
  "running",
  "cancelling",
]);

export function isNativeCalculationActive(status: RunMonitorStatus): boolean {
  return ACTIVE_CALCULATION_STATES.has(status);
}

const projectIsMutable: ContextRule = (context) =>
  context.projectOpen && context.projectWritable && !isNativeCalculationActive(context.calculationStatus);
const modelIsMutable: ContextRule = (context) =>
  context.surface === "model" && context.hasDataset && projectIsMutable(context);
const canOpenCalculation: ContextRule = (context) =>
  context.projectOpen
  && context.projectWritable
  && context.hasDataset
  && !isNativeCalculationActive(context.calculationStatus);
const hasModelSelection: ContextRule = (context) =>
  context.selection.count > 0 && ["construct", "path", "multiple"].includes(context.selection.kind);

/**
 * The single source of truth for native command presentation and semantics.
 * Consumers render placements and dispatch `action`; they do not recreate rules.
 */
export const NATIVE_COMMANDS: readonly NativeCommandDefinition[] = [
  {
    id: "new-project",
    label: "New Project…",
    action: { id: "project.new" },
    shortcut: { key: "n", ctrl: true, allowInEditable: true },
    menu: { menu: "file", order: 10 },
    toolbar: [{ surface: "launcher", order: 10 }],
    enabledWhen: (context) => !isNativeCalculationActive(context.calculationStatus),
  },
  {
    id: "open-project",
    label: "Open Project…",
    action: { id: "project.open" },
    shortcut: { key: "o", ctrl: true, allowInEditable: true },
    menu: { menu: "file", order: 20 },
    toolbar: [{ surface: "launcher", order: 20 }],
    enabledWhen: (context) => !isNativeCalculationActive(context.calculationStatus),
  },
  {
    id: "open-demo-project",
    label: "Open Sample Project",
    action: { id: "project.open-demo" },
    menu: { menu: "file", order: 25 },
    enabledWhen: (context) => !isNativeCalculationActive(context.calculationStatus),
  },
  {
    id: "open-project-item",
    label: (context) => context.selection.kind === "project-data"
      ? "Open Data"
      : context.selection.kind === "project-report"
        ? "Open Report"
        : "Open Model",
    action: { id: "explorer.open-selection" },
    contextMenu: [{
      surface: "launcher",
      selections: ["project-data", "project-model", "project-report"],
      order: 5,
    }],
    visibleWhen: (context) => context.surface === "launcher"
      && ["project-data", "project-model", "project-report"].includes(context.selection.kind),
    enabledWhen: (context) => context.projectOpen,
  },
  {
    id: "new-model",
    label: "New Model…",
    action: { id: "explorer.new-model" },
    shortcut: { key: "n", ctrl: true, shift: true, allowInEditable: true },
    menu: { menu: "file", order: 27 },
    toolbar: [
      { surface: "launcher", order: 30, primary: true },
      { surface: "data", order: 40, primary: true },
    ],
    contextMenu: [{ surface: "launcher", selections: ["project-models", "project-model"], order: 10 }],
    visibleWhen: (context) => context.projectOpen && (
      context.surface === "launcher"
      || (context.surface === "data" && context.hasDataset && !context.canOpenContextModel)
    ),
    enabledWhen: (context) => projectIsMutable(context)
      && (context.surface !== "data" || context.hasDataset),
  },
  {
    id: "save-report",
    label: "Save Report…",
    action: { id: "explorer.save-report" },
    menu: { menu: "file", order: 35 },
    toolbar: [
      { surface: "launcher", order: 40 },
      { surface: "results", order: 25 },
    ],
    contextMenu: [
      { surface: "launcher", selections: ["project-reports"], order: 10 },
      { surface: "results", selections: ["result"], order: 15, separatorBefore: true },
    ],
    visibleWhen: (context) => (context.surface === "launcher" || context.surface === "results")
      && context.projectOpen
      && context.hasCompletedRun
      && !context.selectedResultSaved,
    enabledWhen: projectIsMutable,
  },
  {
    id: "import-data",
    label: "Import Data…",
    action: { id: "project.import-data" },
    shortcut: { key: "i", ctrl: true, allowInEditable: true },
    menu: { menu: "file", order: 30, separatorBefore: true },
    toolbar: [{ surface: "data", order: 10, primary: true }],
    visibleWhen: (context) => context.projectOpen,
    enabledWhen: projectIsMutable,
  },
  {
    id: "recode-variable",
    label: "Recode Variable…",
    action: { id: "data.recode" },
    menu: { menu: "edit", order: 5 },
    toolbar: [{ surface: "data", order: 20 }],
    contextMenu: [{ surface: "data", selections: ["variable"], order: 5 }],
    visibleWhen: (context) => context.surface === "data" && context.selection.kind === "variable",
    enabledWhen: (context) => projectIsMutable(context) && context.canRecode,
  },
  {
    id: "configure-groups",
    label: (context) => context.selectedVariableIsGrouping ? "Edit Groups…" : "Use as Grouping Variable…",
    action: { id: "data.configure-groups" },
    menu: { menu: "edit", order: 7 },
    toolbar: [{ surface: "data", order: 25 }],
    contextMenu: [{ surface: "data", selections: ["variable"], order: 7 }],
    visibleWhen: (context) => context.surface === "data" && context.selection.kind === "variable",
    enabledWhen: (context) => projectIsMutable(context) && context.canConfigureGroups,
  },
  {
    id: "save-project",
    label: "Save",
    action: { id: "project.save" },
    shortcut: { key: "s", ctrl: true, allowInEditable: true },
    menu: { menu: "file", order: 40 },
    toolbar: [
      { surface: "data", order: 30 },
      { surface: "model", order: 10 },
    ],
    visibleWhen: (context) => context.projectOpen,
    enabledWhen: projectIsMutable,
  },
  {
    id: "save-project-as",
    label: "Save As…",
    action: { id: "project.save", saveAs: true },
    shortcut: { key: "s", ctrl: true, shift: true, allowInEditable: true },
    menu: { menu: "file", order: 50 },
    visibleWhen: (context) => context.projectOpen,
    enabledWhen: projectIsMutable,
  },
  {
    id: "rename-project-item",
    label: (context) => context.selection.kind === "project-report" ? "Rename Report…" : "Rename Model…",
    action: { id: "explorer.rename-selection" },
    shortcut: { key: "F2" },
    menu: { menu: "edit", order: 4 },
    contextMenu: [{ surface: "launcher", selections: ["project-model", "project-report"], order: 20, separatorBefore: true }],
    visibleWhen: (context) => context.surface === "launcher"
      && ["project-model", "project-report"].includes(context.selection.kind),
    enabledWhen: projectIsMutable,
  },
  {
    id: "delete-project-item",
    label: (context) => context.selection.kind === "project-report" ? "Remove Report…" : "Delete Model…",
    action: { id: "explorer.delete-selection" },
    shortcut: { key: "Delete" },
    menu: { menu: "edit", order: 6 },
    contextMenu: [{ surface: "launcher", selections: ["project-model", "project-report"], order: 30 }],
    visibleWhen: (context) => context.surface === "launcher"
      && ["project-model", "project-report"].includes(context.selection.kind),
    enabledWhen: projectIsMutable,
  },
  {
    id: "undo",
    label: "Undo",
    action: { id: "model.undo" },
    shortcut: { key: "z", ctrl: true },
    menu: { menu: "edit", order: 10 },
    visibleWhen: (context) => context.surface === "model",
    enabledWhen: (context) => modelIsMutable(context) && context.canUndo,
  },
  {
    id: "redo",
    label: "Redo",
    action: { id: "model.redo" },
    shortcut: { key: "y", ctrl: true },
    menu: { menu: "edit", order: 20 },
    visibleWhen: (context) => context.surface === "model",
    enabledWhen: (context) => modelIsMutable(context) && context.canRedo,
  },
  {
    id: "go-project",
    label: "Project",
    action: { id: "surface.navigate", surface: "launcher" },
    menu: { menu: "view", order: 10 },
  },
  {
    id: "go-data",
    label: "Data",
    action: { id: "surface.navigate", surface: "data" },
    menu: { menu: "view", order: 20 },
    enabledWhen: (context) => context.projectOpen,
  },
  {
    id: "go-model",
    label: (context) => context.surface === "results" ? "Edit Model" : "Model",
    action: { id: "surface.navigate", surface: "model" },
    menu: { menu: "view", order: 30 },
    toolbar: [
      { surface: "data", order: 40 },
      { surface: "results", order: 10 },
    ],
    contextMenu: [
      { surface: "data", selections: ["dataset", "variable"], order: 10 },
      { surface: "results", selections: ["result"], order: 10 },
    ],
    visibleWhen: (context) => !["data", "results"].includes(context.surface)
      || context.canOpenContextModel,
    enabledWhen: (context) => context.projectOpen
      && context.hasDataset
      && context.canOpenContextModel
      && !isNativeCalculationActive(context.calculationStatus),
  },
  {
    id: "go-result-data",
    label: "Edit Data",
    action: { id: "surface.navigate", surface: "data" },
    toolbar: [{ surface: "results", order: 10 }],
    contextMenu: [{ surface: "results", selections: ["result"], order: 10 }],
    visibleWhen: (context) => context.surface === "results" && !context.canOpenContextModel,
    enabledWhen: (context) => context.projectOpen
      && context.hasDataset
      && !isNativeCalculationActive(context.calculationStatus),
  },
  {
    id: "go-results",
    label: "Results",
    action: { id: "surface.navigate", surface: "results" },
    menu: { menu: "view", order: 40 },
    enabledWhen: (context) => context.projectOpen,
  },
  {
    id: "toggle-properties",
    label: (context) => context.propertiesOpen ? "Hide Properties" : "Show Properties",
    action: { id: "view.toggle-properties" },
    menu: { menu: "view", order: 50, separatorBefore: true },
    visibleWhen: (context) => context.surface !== "launcher",
  },
  {
    id: "select-tool",
    label: "Select",
    action: { id: "model.set-tool", tool: "select" },
    shortcut: { key: "v" },
    toolbar: [{ surface: "model", order: 20 }],
    enabledWhen: modelIsMutable,
  },
  {
    id: "pan-tool",
    label: "Pan",
    action: { id: "model.set-tool", tool: "pan" },
    shortcut: { key: "h" },
    toolbar: [{ surface: "model", order: 30 }],
    enabledWhen: modelIsMutable,
  },
  {
    id: "add-construct",
    label: "Construct",
    action: { id: "model.add-construct" },
    shortcut: { key: "c" },
    toolbar: [{ surface: "model", order: 40 }],
    enabledWhen: modelIsMutable,
  },
  {
    id: "path-tool",
    label: "Path",
    action: { id: "model.set-tool", tool: "path" },
    shortcut: { key: "p" },
    toolbar: [{ surface: "model", order: 50 }],
    enabledWhen: modelIsMutable,
  },
  {
    id: "add-higher-order",
    label: "Higher-Order Construct…",
    action: { id: "model.add-higher-order" },
    toolbar: [{ surface: "model", order: 53 }],
    contextMenu: [{ surface: "model", selections: ["multiple"], order: 6 }],
    enabledWhen: (context) => modelIsMutable(context) && context.canAddHigherOrder,
  },
  {
    id: "add-moderating-effect",
    label: "Moderating Effect…",
    action: { id: "model.add-moderating-effect" },
    shortcut: { key: "m" },
    toolbar: [{ surface: "model", order: 55 }],
    contextMenu: [{ surface: "model", selections: ["path"], order: 7 }],
    enabledWhen: (context) => modelIsMutable(context) && context.canAddModeration,
  },
  {
    id: "edit-selection",
    label: (context) => context.selection.kind === "path" ? "Edit Path Properties…" : "Edit Construct Properties…",
    action: { id: "model.edit-selection" },
    shortcut: { key: "enter" },
    menu: { menu: "edit", order: 30, separatorBefore: true },
    contextMenu: [{ surface: "model", selections: ["construct", "path"], order: 5 }],
    visibleWhen: (context) => context.surface === "model" && ["construct", "path"].includes(context.selection.kind),
    enabledWhen: modelIsMutable,
  },
  {
    id: "delete-selection",
    label: "Delete Selection",
    action: { id: "model.delete-selection" },
    shortcut: { key: "delete" },
    contextMenu: [{ surface: "model", selections: ["construct", "path", "multiple"], order: 10 }],
    visibleWhen: (context) => context.surface === "model" && hasModelSelection(context),
    enabledWhen: (context) => modelIsMutable(context) && hasModelSelection(context),
  },
  {
    id: "arrange-model",
    label: "Arrange",
    action: { id: "model.arrange", strategy: "smartpls" },
    toolbar: [{ surface: "model", order: 60 }],
    contextMenu: [{ surface: "model", order: 20, separatorBefore: true }],
    enabledWhen: modelIsMutable,
  },
  {
    id: "fit-model",
    label: "Fit",
    action: { id: "model.fit" },
    shortcut: { key: "f" },
    toolbar: [{ surface: "model", order: 70 }],
    contextMenu: [{ surface: "model", order: 30 }],
    enabledWhen: (context) => context.surface === "model" && context.hasDataset,
  },
  {
    id: "open-calculation",
    label: (context) => context.surface === "data" ? "Analyze…" : "Calculate…",
    action: { id: "calculation.open" },
    shortcut: { key: "r", ctrl: true },
    menu: { menu: "calculate", order: 10 },
    toolbar: [
      { surface: "data", order: 50, primary: true },
      { surface: "model", order: 80, primary: true },
      { surface: "results", order: 20 },
    ],
    contextMenu: [{ surface: "model", order: 40, separatorBefore: true }],
    visibleWhen: (context) => context.surface === "data" || context.surface === "model" || context.surface === "results",
    enabledWhen: canOpenCalculation,
  },
  {
    id: "cancel-calculation",
    label: "Cancel Calculation",
    action: { id: "calculation.cancel" },
    shortcut: { key: "escape" },
    menu: { menu: "calculate", order: 20, separatorBefore: true },
    visibleWhen: (context) => isNativeCalculationActive(context.calculationStatus),
    enabledWhen: (context) => ["queued", "validating", "running"].includes(context.calculationStatus),
  },
  {
    id: "export-results",
    label: "Export…",
    action: { id: "results.export" },
    shortcut: { key: "e", ctrl: true },
    toolbar: [{ surface: "results", order: 30, primary: true }],
    contextMenu: [{ surface: "results", selections: ["result"], order: 20, separatorBefore: true }],
    visibleWhen: (context) => context.surface === "results",
    enabledWhen: (context) => context.hasCompletedRun,
  },
  {
    id: "run-details",
    label: "Run Details and Log…",
    action: { id: "results.open-run-details" },
    menu: { menu: "tools", order: 15, separatorBefore: true },
    toolbar: [{ surface: "results", order: 40 }],
    contextMenu: [{ surface: "results", selections: ["result"], order: 30 }],
    visibleWhen: (context) => context.surface === "results",
    enabledWhen: (context) => context.hasCompletedRun,
  },
  {
    id: "method-scope",
    label: "Validation and Method Scope…",
    action: { id: "utility.open", utility: "method-scope" },
    menu: { menu: "tools", order: 10 },
  },
  {
    id: "preferences",
    label: "Preferences…",
    action: { id: "utility.open", utility: "preferences" },
    menu: { menu: "tools", order: 20 },
    enabledWhen: (context) => !isNativeCalculationActive(context.calculationStatus),
  },
  {
    id: "keyboard-shortcuts",
    label: "Keyboard Shortcuts",
    action: { id: "utility.open", utility: "shortcuts" },
    menu: { menu: "help", order: 10 },
  },
  {
    id: "about",
    label: "About QuickPLS",
    action: { id: "utility.open", utility: "about" },
    menu: { menu: "help", order: 20 },
  },
] as const;

export interface ResolvedNativeCommand extends Omit<NativeCommandDefinition, "label"> {
  label: string;
  visible: boolean;
  enabled: boolean;
}

const commandById = new Map<NativeCommandId, NativeCommandDefinition>(
  NATIVE_COMMANDS.map((command) => [command.id, command]),
);

export function resolveNativeCommand(
  id: NativeCommandId,
  context: Readonly<NativeCommandContext>,
): ResolvedNativeCommand {
  const command = commandById.get(id);
  if (!command) throw new Error(`Unknown native command: ${id}`);
  const visible = command.visibleWhen?.(context) ?? true;
  return {
    ...command,
    label: typeof command.label === "function" ? command.label(context) : command.label,
    visible,
    enabled: visible && (command.enabledWhen?.(context) ?? true),
  };
}

export type NativeCommandPlacementQuery =
  | { kind: "menu"; menu: NativeMenuId }
  | { kind: "toolbar"; surface: NativeSurface }
  | { kind: "context-menu"; surface: NativeSurface };

function placementOrder(command: NativeCommandDefinition, query: NativeCommandPlacementQuery): number | null {
  if (query.kind === "menu") return command.menu?.menu === query.menu ? command.menu.order : null;
  if (query.kind === "toolbar") {
    return command.toolbar?.find((placement) => placement.surface === query.surface)?.order ?? null;
  }
  return command.contextMenu?.find((placement) => placement.surface === query.surface)?.order ?? null;
}

function isContextPlacementVisible(
  command: NativeCommandDefinition,
  query: NativeCommandPlacementQuery,
  context: Readonly<NativeCommandContext>,
): boolean {
  if (query.kind !== "context-menu") return true;
  const placement = command.contextMenu?.find((candidate) => candidate.surface === query.surface);
  return Boolean(placement && (!placement.selections || placement.selections.includes(context.selection.kind)));
}

export function nativeCommandsFor(
  query: NativeCommandPlacementQuery,
  context: Readonly<NativeCommandContext>,
): ResolvedNativeCommand[] {
  return NATIVE_COMMANDS
    .map((command) => ({ command, order: placementOrder(command, query) }))
    .filter((entry): entry is { command: NativeCommandDefinition; order: number } => entry.order !== null)
    .filter(({ command }) => isContextPlacementVisible(command, query, context))
    .map(({ command, order }) => ({ resolved: resolveNativeCommand(command.id, context), order }))
    .filter(({ resolved }) => resolved.visible)
    .sort((left, right) => left.order - right.order)
    .map(({ resolved }) => resolved);
}

/**
 * Resolves a workspace context menu from the same typed registry for both
 * pointer and keyboard invocation. Pointer callers may snapshot the object
 * selected by the right-click before React has rendered the store update.
 */
export function nativeContextMenuCommands(
  context: Readonly<NativeCommandContext>,
  selection: Readonly<NativeCommandContext["selection"]> = context.selection,
): ResolvedNativeCommand[] {
  return nativeCommandsFor(
    { kind: "context-menu", surface: context.surface },
    { ...context, selection: { ...selection } },
  );
}

export interface NativeShortcutInput {
  key: string;
  ctrlKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  metaKey?: boolean;
  editable?: boolean;
}

function shortcutMatches(shortcut: NativeKeyboardShortcut, input: NativeShortcutInput): boolean {
  return shortcut.key.toLowerCase() === input.key.toLowerCase()
    && Boolean(shortcut.ctrl) === Boolean(input.ctrlKey)
    && Boolean(shortcut.shift) === Boolean(input.shiftKey)
    && Boolean(shortcut.alt) === Boolean(input.altKey)
    && Boolean(shortcut.meta) === Boolean(input.metaKey);
}

export function nativeCommandForShortcut(
  input: NativeShortcutInput,
  context: Readonly<NativeCommandContext>,
): ResolvedNativeCommand | null {
  for (const command of NATIVE_COMMANDS) {
    if (!command.shortcut || !shortcutMatches(command.shortcut, input)) continue;
    const resolved = resolveNativeCommand(command.id, context);
    if (!resolved.visible) continue;
    if (input.editable && !command.shortcut.allowInEditable) return null;
    return resolved.enabled ? resolved : null;
  }
  return null;
}

export function formatNativeShortcut(shortcut: NativeKeyboardShortcut | undefined): string | undefined {
  if (!shortcut) return undefined;
  const key = shortcut.key.length === 1
    ? shortcut.key.toUpperCase()
    : shortcut.key[0].toUpperCase() + shortcut.key.slice(1);
  return [
    shortcut.ctrl ? "Ctrl" : null,
    shortcut.alt ? "Alt" : null,
    shortcut.shift ? "Shift" : null,
    shortcut.meta ? "Meta" : null,
    key,
  ].filter(Boolean).join("+");
}

export function executeNativeCommand(
  id: NativeCommandId,
  context: Readonly<NativeCommandContext>,
  dispatch: (action: NativeCommandAction) => void,
): boolean {
  const command = resolveNativeCommand(id, context);
  if (!command.visible || !command.enabled) return false;
  dispatch(command.action);
  return true;
}
