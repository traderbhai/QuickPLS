import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const app = () => readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
const styles = () => readFileSync("src/native/nativeDesktop.css", "utf8");

describe("native command accessibility contracts", () => {
  it("renders registry-derived workspace context menus for pointer and Windows keyboard gestures", () => {
    const source = app();
    const canvas = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain("nativeContextMenuCommands(");
    expect(source).toContain("contextMenu?.selection");
    expect(source).toContain("canAddModeration: contextMenu.canAddModeration");
    expect(source).toContain("const onModelCanvasContextMenuRequest");
    expect(source).toContain('request.target.kind === "path" ? canAddNativeModeration(nodes, edges, request.target.id) : false');
    expect(source).toContain("<ModelCanvas onContextMenuRequest={onContextMenuRequest} showGeneratedInteractionTerms={showGeneratedInteractionTerms} />");
    expect(source).toContain('label: "Add Second Moderator…"');
    expect(canvas).toContain('target: { kind: "parent_interaction", interactionTermId: selectedInteractionTermId }');
    expect(canvas).toContain("requestNativeContextMenu(event");
    expect(canvas).toContain("event.stopPropagation()");
    expect(source).toContain("onContextMenu={onWorkspaceContextMenu}");
    expect(source).toContain("onKeyDown={onWorkspaceKeyDown}");
    expect(source).toContain("isContextMenuKeyboardGesture(event.key, event.shiftKey)");
    expect(source).not.toContain('surface === "model" && target.closest(".nd-canvas-host")');
    expect(source).toContain('className="nd-context-menu"');
    expect(source).toContain('role="menu"');
    expect(source).toContain('role="menuitem"');
    expect(source).toContain('Unavailable: ${item.disabledReason}');
    expect(source).toContain('title={item.disabled ? item.disabledReason : undefined}');
    expect(source).toContain('Unavailable: ${command.disabledReason}');
    expect(source).toContain('title={command.disabled ? command.disabledReason ?? command.label : command.label}');
  });

  it("uses the native properties pane for model editing instead of browser prompts", () => {
    const source = app();
    const canvas = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain('case "model.edit-selection"');
    expect(source).toContain("if (!focusModelSelectionEditor()) window.setTimeout(focusModelSelectionEditor, 0)");
    expect(source).toContain("editor.focus({ preventScroll: true })");
    expect(source).toContain('"nd-model-construct-name"');
    expect(source).toContain('"nd-model-path-label"');
    expect(canvas).not.toContain("window.prompt(");
    expect(canvas).not.toContain('className="diagram-context-menu"');
    expect(canvas).not.toContain("Rename indicator label");
  });

  it("implements Windows-style menubar traversal and restores focus on Escape", () => {
    const source = app();
    expect(source).toContain('role="menubar"');
    expect(source).toContain('event.key === "ArrowLeft" || event.key === "ArrowRight"');
    expect(source).toContain('event.key === "ArrowDown" || event.key === "ArrowUp"');
    expect(source).toContain('event.key === "Home" || event.key === "End"');
    expect(source).toContain("restoreTrigger(index)");
    expect(source).toContain("state.returnFocus.focus({ preventScroll: true })");
    expect(source).toMatch(/focusReturnTarget\(\);\s*close\(\);/);
    expect(source).not.toMatch(/setTimeout\(\(\) => \{ if \(state\.returnFocus/);
  });

  it("exposes the live Select, Pan, and Path tool state visually and semantically", () => {
    const source = app();
    expect(source).toContain("const diagramTool = useWorkspace((state) => state.diagramTool)");
    expect(source).toContain('command.action.id === "model.set-tool" ? diagramTool === command.action.tool');
    expect(source).toContain("aria-pressed={command.pressed}");
    expect(styles()).toContain('.nd-command-list button[aria-pressed="true"]');
  });

  it("uses a searchable keyboard-operable method list and native form validation", () => {
    const source = readFileSync("src/native/NativeCalculationDialog.tsx", "utf8");
    expect(source).toContain('role="listbox"');
    expect(source).toContain('role="option"');
    expect(source).toContain('aria-selected={selected}');
    expect(source).toContain('event.key === "Home"');
    expect(source).toContain('event.key === "End"');
    expect(source).toContain('event.key.toLocaleLowerCase() !== "f"');
    expect(source).toContain('role="status" aria-live="polite"');
    expect(source).toContain('<form');
    expect(source).toContain('className="nd-calculation-dialog"');
    expect(source).toContain('className="primary" type="submit" disabled={!canStart}');
    expect(source).toContain('aria-label={`${methodLabel} progress`}');
    expect(styles()).toContain('grid-template-columns: var(--nd-method-browser-width) minmax(0, 1fr)');
  });
});
