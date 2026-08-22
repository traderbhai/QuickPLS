import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const table = readFileSync("src/native/NativeSemParameterTable.tsx", "utf8");
const editor = readFileSync("src/native/NativeSemParameterEditor.tsx", "utf8");

describe("native SEM parameter-table UI contracts", () => {
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
