import type { DesktopMenuId } from "../types";

export interface DesktopCommandDefinition {
  id: string;
  label: string;
  menu: DesktopMenuId;
  shortcut?: string;
  description: string;
  requiresReasonWhenDisabled: boolean;
}

export const DESKTOP_MENU_ORDER: Array<{ id: DesktopMenuId; label: string }> = [
  { id: "file", label: "File" },
  { id: "edit", label: "Edit" },
  { id: "data", label: "Data" },
  { id: "model", label: "Model" },
  { id: "calculate", label: "Calculate" },
  { id: "results", label: "Results" },
  { id: "report", label: "Report" },
  { id: "view", label: "View" },
  { id: "tools", label: "Tools" },
  { id: "window", label: "Window" },
  { id: "help", label: "Help" },
];

export const DESKTOP_COMMANDS: DesktopCommandDefinition[] = [
  { id: "file.new", label: "New Project...", menu: "file", shortcut: "Ctrl+N", description: "Create a blank QuickPLS project.", requiresReasonWhenDisabled: false },
  { id: "file.open", label: "Open Project...", menu: "file", shortcut: "Ctrl+O", description: "Open a .qpls project.", requiresReasonWhenDisabled: false },
  { id: "file.save", label: "Save Project", menu: "file", shortcut: "Ctrl+S", description: "Save the active project.", requiresReasonWhenDisabled: true },
  { id: "file.import", label: "Import Data...", menu: "file", description: "Import raw, covariance, or correlation data.", requiresReasonWhenDisabled: false },
  { id: "file.export", label: "Export Options...", menu: "file", description: "Open report, table, and diagram export options.", requiresReasonWhenDisabled: true },
  { id: "edit.undo", label: "Undo", menu: "edit", shortcut: "Ctrl+Z", description: "Undo the last diagram edit.", requiresReasonWhenDisabled: true },
  { id: "edit.redo", label: "Redo", menu: "edit", shortcut: "Ctrl+Y", description: "Redo the last diagram edit.", requiresReasonWhenDisabled: true },
  { id: "data.import", label: "Import Data...", menu: "data", description: "Open the desktop import flow.", requiresReasonWhenDisabled: false },
  { id: "model.open", label: "Open SEM Designer", menu: "model", description: "Open the model workbench.", requiresReasonWhenDisabled: false },
  { id: "model.add_construct", label: "Add Latent Construct", menu: "model", description: "Add a construct to the SEM diagram.", requiresReasonWhenDisabled: false },
  { id: "model.arrange", label: "Arrange Like SmartPLS", menu: "model", description: "Apply the SmartPLS-style diagram layout.", requiresReasonWhenDisabled: true },
  { id: "model.validate", label: "Validate Diagram", menu: "model", description: "Check model structure and diagram readiness.", requiresReasonWhenDisabled: true },
  { id: "calculate.setup", label: "Calculation Setup...", menu: "calculate", shortcut: "F4", description: "Review method settings and expected outputs.", requiresReasonWhenDisabled: false },
  { id: "calculate.run", label: "Run Selected Method", menu: "calculate", shortcut: "F5", description: "Run the selected method when readiness checks pass.", requiresReasonWhenDisabled: true },
  { id: "calculate.cancel", label: "Cancel Running Job", menu: "calculate", shortcut: "Esc", description: "Cancel the active calculation job.", requiresReasonWhenDisabled: true },
  { id: "results.open", label: "Open Results", menu: "results", description: "Open saved runs and result workbooks.", requiresReasonWhenDisabled: false },
  { id: "results.copy_run_list", label: "Copy Run List", menu: "results", description: "Copy run provenance rows to the clipboard.", requiresReasonWhenDisabled: true },
  { id: "results.export_table", label: "Export Current Table", menu: "results", description: "Export the selected result table.", requiresReasonWhenDisabled: true },
  { id: "report.open", label: "Open Publication Report", menu: "report", description: "Open the report/export workspace.", requiresReasonWhenDisabled: false },
  { id: "report.export", label: "Export Report...", menu: "report", description: "Export report tables and diagram artifacts.", requiresReasonWhenDisabled: true },
  { id: "view.focus_diagram", label: "Focus Diagram", menu: "view", shortcut: "Ctrl+Shift+F", description: "Collapse side panes for focused modeling.", requiresReasonWhenDisabled: false },
  { id: "tools.trust_center", label: "Methods & References", menu: "tools", description: "Review method requirements, references, and known limitations.", requiresReasonWhenDisabled: false },
  { id: "tools.preferences", label: "Preferences...", menu: "tools", description: "Open QuickPLS desktop preferences.", requiresReasonWhenDisabled: false },
  { id: "window.home", label: "Home", menu: "window", description: "Switch to the Home workspace.", requiresReasonWhenDisabled: false },
  { id: "window.data", label: "Data", menu: "window", description: "Switch to the Data workbench.", requiresReasonWhenDisabled: false },
  { id: "window.model", label: "Model", menu: "window", description: "Switch to the Model workbench.", requiresReasonWhenDisabled: false },
  { id: "help.shortcuts", label: "Keyboard Shortcuts...", menu: "help", shortcut: "?", description: "Show keyboard shortcuts.", requiresReasonWhenDisabled: false },
];

export function desktopCommandsForMenu(menu: DesktopMenuId) {
  return DESKTOP_COMMANDS.filter((command) => command.menu === menu);
}
