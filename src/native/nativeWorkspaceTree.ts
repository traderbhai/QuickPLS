import type {
  NativeCanonicalModelSpec,
  NativeExplorerSelection,
  NativeSavedReport,
} from "../types";

export type NativeWorkspaceTreeNodeKind =
  | "project"
  | "data"
  | "models"
  | "model"
  | "reports"
  | "report";

export interface NativeWorkspaceTreeNode {
  id: string;
  kind: NativeWorkspaceTreeNodeKind;
  label: string;
  level: 1 | 2 | 3;
  parentId: string | null;
  expandable: boolean;
  expanded: boolean;
  modelId?: string;
  resultId?: string;
}

export interface NativeWorkspaceTreeInput {
  projectName: string;
  datasetName: string;
  models: readonly Pick<NativeCanonicalModelSpec, "id" | "name">[];
  reports: readonly Pick<NativeSavedReport, "resultId" | "name">[];
  expandedIds: ReadonlySet<string>;
}

export type NativeWorkspaceTreeNavigationKey =
  | "ArrowDown"
  | "ArrowUp"
  | "ArrowRight"
  | "ArrowLeft"
  | "Home"
  | "End";

export interface NativeWorkspaceTreeNavigation {
  focusId: string;
  expansion?: { id: string; expanded: boolean };
}

const PROJECT_ID = "project";
const DATA_ID = "data";
const MODELS_ID = "models";
const REPORTS_ID = "reports";

export function nativeWorkspaceModelTreeId(modelId: string): string {
  return `model:${modelId}`;
}

export function nativeWorkspaceReportTreeId(resultId: string): string {
  return `report:${resultId}`;
}

export function nextNativeWorkspaceModelName(
  models: readonly Pick<NativeCanonicalModelSpec, "name">[],
): string {
  const names = new Set(models.map((model) => model.name.trim().normalize("NFKC").toLowerCase()));
  let suffix = 1;
  while (names.has(`model ${suffix}`)) suffix += 1;
  return `Model ${suffix}`;
}

/** Builds only the visible rows so roving focus cannot land on a collapsed child. */
export function buildNativeWorkspaceTree(input: NativeWorkspaceTreeInput): NativeWorkspaceTreeNode[] {
  const projectExpanded = input.expandedIds.has(PROJECT_ID);
  const modelsExpanded = input.expandedIds.has(MODELS_ID);
  const reportsExpanded = input.expandedIds.has(REPORTS_ID);
  const rows: NativeWorkspaceTreeNode[] = [{
    id: PROJECT_ID,
    kind: "project",
    label: input.projectName,
    level: 1,
    parentId: null,
    expandable: true,
    expanded: projectExpanded,
  }];

  if (!projectExpanded) return rows;

  rows.push({
    id: DATA_ID,
    kind: "data",
    label: input.datasetName || "Data",
    level: 2,
    parentId: PROJECT_ID,
    expandable: false,
    expanded: false,
  });
  rows.push({
    id: MODELS_ID,
    kind: "models",
    label: "Models",
    level: 2,
    parentId: PROJECT_ID,
    expandable: input.models.length > 0,
    expanded: modelsExpanded,
  });

  if (modelsExpanded) {
    const sortedModels = [...input.models].sort((left, right) => compareTreeLabels(left.name, left.id, right.name, right.id));
    rows.push(...sortedModels.map<NativeWorkspaceTreeNode>((model) => ({
      id: nativeWorkspaceModelTreeId(model.id),
      kind: "model",
      label: model.name,
      level: 3,
      parentId: MODELS_ID,
      expandable: false,
      expanded: false,
      modelId: model.id,
    })));
  }

  rows.push({
    id: REPORTS_ID,
    kind: "reports",
    label: "Reports",
    level: 2,
    parentId: PROJECT_ID,
    expandable: input.reports.length > 0,
    expanded: reportsExpanded,
  });

  if (reportsExpanded) {
    const sortedReports = [...input.reports].sort((left, right) => compareTreeLabels(left.name, left.resultId, right.name, right.resultId));
    rows.push(...sortedReports.map<NativeWorkspaceTreeNode>((report) => ({
      id: nativeWorkspaceReportTreeId(report.resultId),
      kind: "report",
      label: report.name,
      level: 3,
      parentId: REPORTS_ID,
      expandable: false,
      expanded: false,
      resultId: report.resultId,
    })));
  }

  return rows;
}

function compareTreeLabels(leftLabel: string, leftId: string, rightLabel: string, rightId: string): number {
  const left = leftLabel.trim().toLowerCase();
  const right = rightLabel.trim().toLowerCase();
  if (left < right) return -1;
  if (left > right) return 1;
  return leftId < rightId ? -1 : leftId > rightId ? 1 : 0;
}

export function nativeWorkspaceTreeIdForSelection(selection: NativeExplorerSelection): string {
  switch (selection.kind) {
    case "project": return PROJECT_ID;
    case "data": return DATA_ID;
    case "models": return MODELS_ID;
    case "model": return nativeWorkspaceModelTreeId(selection.modelId);
    case "reports": return REPORTS_ID;
    case "report": return nativeWorkspaceReportTreeId(selection.resultId);
  }
}

export function nativeWorkspaceSelectionForNode(
  node: Pick<NativeWorkspaceTreeNode, "kind" | "modelId" | "resultId">,
): NativeExplorerSelection | null {
  switch (node.kind) {
    case "project": return { kind: "project" };
    case "data": return { kind: "data" };
    case "models": return { kind: "models" };
    case "model": return node.modelId ? { kind: "model", modelId: node.modelId } : null;
    case "reports": return { kind: "reports" };
    case "report": return node.resultId ? { kind: "report", resultId: node.resultId } : null;
  }
}

export function nativeWorkspaceTreeNavigation(
  nodes: readonly NativeWorkspaceTreeNode[],
  currentId: string,
  key: NativeWorkspaceTreeNavigationKey,
): NativeWorkspaceTreeNavigation | null {
  if (!nodes.length) return null;
  const currentIndex = Math.max(0, nodes.findIndex((node) => node.id === currentId));
  const current = nodes[currentIndex];
  if (!current) return null;

  if (key === "Home") return { focusId: nodes[0].id };
  if (key === "End") return { focusId: nodes[nodes.length - 1].id };
  if (key === "ArrowDown") return { focusId: nodes[Math.min(nodes.length - 1, currentIndex + 1)].id };
  if (key === "ArrowUp") return { focusId: nodes[Math.max(0, currentIndex - 1)].id };

  if (key === "ArrowRight") {
    if (current.expandable && !current.expanded) {
      return { focusId: current.id, expansion: { id: current.id, expanded: true } };
    }
    const firstChild = nodes.slice(currentIndex + 1).find((node) => node.parentId === current.id);
    return { focusId: firstChild?.id ?? current.id };
  }

  if (current.expandable && current.expanded) {
    return { focusId: current.id, expansion: { id: current.id, expanded: false } };
  }
  return { focusId: current.parentId ?? current.id };
}
