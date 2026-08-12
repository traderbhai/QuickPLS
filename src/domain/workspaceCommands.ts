export type WorkspaceCommandEvent =
  | "quickpls:run-analysis"
  | "quickpls:save-project"
  | "quickpls:open-project"
  | "quickpls:open-demo-project"
  | "quickpls:import-data";

export const workspaceCommandEvents: WorkspaceCommandEvent[] = [
  "quickpls:run-analysis",
  "quickpls:save-project",
  "quickpls:open-project",
  "quickpls:open-demo-project",
  "quickpls:import-data",
];

export function dispatchWorkspaceCommand(event: WorkspaceCommandEvent) {
  window.dispatchEvent(new CustomEvent(event));
}
