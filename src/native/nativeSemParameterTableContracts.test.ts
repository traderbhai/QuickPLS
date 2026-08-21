import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const app = readFileSync("src/native/NativeDesktopApp.tsx", "utf8");
const table = readFileSync("src/native/NativeSemParameterTable.tsx", "utf8");
const editor = readFileSync("src/native/NativeSemParameterEditor.tsx", "utf8");

describe("native SEM parameter-table UI contracts", () => {
  it("provides an operable tablist and labelled panels", () => {
    expect(app).toContain('role="tablist"');
    expect(app).toContain('id="nd-model-canvas-tab"');
    expect(app).toContain('id="nd-model-parameter-tab"');
    expect(app).toContain('aria-selected={documentView === "parameters"}');
    expect(app).toContain('["ArrowLeft", "ArrowRight", "Home", "End"]');
    expect(table).toContain('role="tabpanel"');
    expect(table).toContain('aria-labelledby="nd-model-parameter-tab"');
  });

  it("derives editable rows from the estimator-authoritative SemModelV4 adapter", () => {
    expect(table).toContain("projectNativeWorkbenchSemParameterTableV4");
    expect(table).toContain("part of the resident SemModelV4 authority used by native preflight and compatible estimators");
    expect(table).not.toContain("nd-experimental-chip");
    expect(app).toContain("generalSemViewAvailable && documentView === \"parameters\"");
    expect(table).toContain("NativeSemParameterEditor");
    expect(table).toContain("NativeSemVariableEditor");
    expect(table).not.toContain("contentEditable");
  });

  it("uses accessible table semantics and returns focusable rows to their canvas source", () => {
    expect(table).toContain('<caption className="nd-sr-only">');
    expect(table).toContain('<th scope="col">Object</th>');
    expect(table).toContain('scope="rowgroup"');
    expect(table).toContain('scope="row"');
    expect(table).toContain('aria-label={`Edit ${row.label}`}');
    expect(table).toContain('aria-expanded={editingRowId === row.id}');
    expect(table).toContain('document.getElementById(triggerId)?.focus()');
    expect(table).toContain('setSelectedNode(source.id)');
    expect(table).toContain('setSelectedEdge(source.id)');
    expect(table).toContain('new CustomEvent(eventName!');
  });

  it("supports keyboard-contained, explicitly labelled editor forms", () => {
    expect(editor).toContain('aria-labelledby={`${id}-title`}');
    expect(editor).toContain('event.key !== "Escape"');
    expect(editor).toContain("Start value");
    expect(editor).toContain("Lower bound");
    expect(editor).toContain("Upper bound");
    expect(editor).toContain("Equality label");
    expect(editor).toContain("Factor identification");
    expect(editor).toContain("Estimate ordinal thresholds");
  });
});
