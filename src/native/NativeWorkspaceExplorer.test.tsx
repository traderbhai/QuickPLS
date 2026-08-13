import { readFileSync } from "node:fs";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { NativeCanonicalModelSpec, NativeSavedReport } from "../types";
import NativeWorkspaceExplorer, { NativeWorkspaceExplorerDialog } from "./NativeWorkspaceExplorer";

const model: NativeCanonicalModelSpec = {
  id: "model-a",
  name: "Customer loyalty",
  constructs: [
    { id: "x", name: "Quality", short_name: "QUAL", mode: "reflective", indicators: ["qual1", "qual2"] },
    { id: "y", name: "Loyalty", short_name: "LOY", mode: "reflective", indicators: ["loy1", "loy2"] },
  ],
  paths: [{ source: "x", target: "y" }],
  controls: [],
  higher_order_constructs: [],
  interactions: [],
};
const report: NativeSavedReport = {
  resultId: "run-a",
  name: "PLS Algorithm",
  savedAt: "2026-08-11T00:00:00.000Z",
};

function explorerMarkup(projectWritable = true, calculationStatus: "idle" | "running" = "idle") {
  return renderToStaticMarkup(<NativeWorkspaceExplorer
    projectName="Corporate reputation"
    projectPath="C:\\Projects\\corporate-reputation.qpls"
    projectWritable={projectWritable}
    datasetName="corporate-reputation.csv"
    datasetRows={344}
    datasetColumns={21}
    models={[model]}
    activeModelId={model.id}
    reports={[report]}
    selection={{ kind: "model", modelId: model.id }}
    currentResultId="run-b"
    currentResultName="Bootstrapping"
    currentResultSaved={false}
    calculationStatus={calculationStatus}
    onSelectionChange={vi.fn()}
    onOpenData={vi.fn()}
    onOpenModel={vi.fn()}
    onOpenReport={vi.fn()}
    onCreateModel={vi.fn()}
    onRenameModel={vi.fn()}
    onDeleteModel={vi.fn()}
    onSaveReport={vi.fn()}
    onRenameReport={vi.fn()}
    onRemoveReport={vi.fn()}
  />);
}

describe("NativeWorkspaceExplorer", () => {
  it("renders an expanded Windows-style tree with one selected roving tab stop", () => {
    const markup = explorerMarkup();
    expect(markup).toContain('role="tree"');
    expect(markup).toContain('role="treeitem"');
    expect(markup).toContain('aria-level="1"');
    expect(markup).toContain('aria-level="2"');
    expect(markup).toContain('aria-level="3"');
    expect(markup).toContain('aria-expanded="true"');
    expect(markup).toContain('aria-selected="true"');
    expect(markup.match(/tabindex="0"/g)).toHaveLength(1);
    expect(markup).toContain("Customer loyalty");
    expect(markup).toContain("Active model");
    expect(markup).not.toContain("Recent projects");
    expect(markup).not.toContain("nd-explorer-title-actions");
  });

  it.each([
    [false, "idle"],
    [true, "running"],
  ] as const)("disables mutation actions when writable=%s and status=%s", (projectWritable, calculationStatus) => {
    const markup = explorerMarkup(projectWritable, calculationStatus);
    expect(markup).toMatch(/<button[^>]*disabled=""[^>]*>.*Rename<\/button>/);
    expect(markup).toMatch(/<button[^>]*class="danger"[^>]*disabled=""[^>]*>.*Delete<\/button>/);
  });

  it("routes Enter, F2, Delete, and Shift+F10 through the tree and registry contracts", () => {
    const source = readFileSync("src/native/NativeWorkspaceExplorer.tsx", "utf8");
    expect(source).toContain('event.key === "Enter"');
    expect(source).toContain('event.key === "F2"');
    expect(source).toContain('event.key === "Delete"');
    expect(source).toContain('event.key === "F10" && event.shiftKey');
    expect(source).toContain('command.action.id === "explorer.rename-selection"');
    expect(source).toContain("if (rename?.enabled) beginRename");
    expect(source).toContain('command.action.id === "explorer.delete-selection"');
    expect(source).toContain("if (remove?.enabled) beginDelete");
  });
});

describe("NativeWorkspaceExplorerDialog", () => {
  it("renders a focused destructive confirmation that preserves completed results", () => {
    const markup = renderToStaticMarkup(<NativeWorkspaceExplorerDialog
      dialog={{ kind: "delete-model", modelId: model.id, name: model.name }}
      close={vi.fn()}
      onCreateModel={vi.fn()}
      onRenameModel={vi.fn()}
      onDeleteModel={vi.fn()}
      onSaveReport={vi.fn()}
      onRenameReport={vi.fn()}
      onRemoveReport={vi.fn()}
    />);
    expect(markup).toContain('role="dialog"');
    expect(markup).toContain('aria-modal="true"');
    expect(markup).toContain("Completed results remain in run history");
    expect(markup).toContain('class="danger"');
  });

  it("keeps async mutations busy, blocks dismissal, and retains errors in the dialog", () => {
    const source = readFileSync("src/native/NativeWorkspaceExplorer.tsx", "utf8");
    expect(source).toContain("setBusy(true)");
    expect(source).toContain("await onCreateModel(name)");
    expect(source).toContain("await onDeleteModel(dialog.modelId)");
    expect(source).toContain("if (event.key === \"Escape\" && !busy)");
    expect(source).toContain("event.target === event.currentTarget) close()");
    expect(source).toContain("setError(reason instanceof Error ? reason.message");
    expect(source).toContain('root?.querySelector<HTMLInputElement>("input")');
    expect(source).toContain("setBusy(false)");
  });
});
