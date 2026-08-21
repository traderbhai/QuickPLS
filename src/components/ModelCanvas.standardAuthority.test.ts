import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

describe("ModelCanvas strict Standard authority routing", () => {
  it("routes scientific canvas actions to typed authority intents while retaining layout actions", () => {
    const source = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain("commitStandardSemModelV4Intent");
    for (const kind of ["add_construct", "add_relationship", "replace_relationship", "delete_construct", "delete_relationship", "assign_indicators"]) {
      expect(source).toContain(`kind: \"${kind}\"`);
    }
    expect(source).toContain('changes.filter((change) => change.type !== "remove")');
    expect(source).toContain("if (strictAuthority) return;");
    expect(source).toContain("moveIndicator(indicator.constructId, indicator.indicator, node.position)");
    expect(source).toContain("StandardSemPresentationLayer");
    expect(source).toContain("standardSemPresentation: presentation");
    expect(source).toContain("if (!strictAuthority || !canEditLayout) return;");
    expect(source).not.toContain('commitStrict({ kind: "caption"');
  });

  it("preserves the canonical latent-control role when a strict path is reconnected", () => {
    const source = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain('relation?.kind === "structural" && relation.role === "control"');
    expect(source).toContain('? { kind: "control", source: connection.source, target: connection.target, label }');
    expect(source).toContain(': { kind: "structural", source: connection.source, target: connection.target, label }');
  });

  it("provides a non-editing Results presentation without borrowing a legacy run overlay", () => {
    const source = readFileSync("src/components/ModelCanvas.tsx", "utf8");
    expect(source).toContain('presentation?: "editor" | "results_readonly";');
    expect(source).toContain('readOnlyResultsPresentation ? "smartpls_result" : diagramMode');
    expect(source).toContain("readOnlyResultsPresentation ? undefined : selectedResultRun");
    expect(source).toContain("if (readOnlyResultsPresentation) return;");
    expect(source).toContain("data-model-canvas-presentation={presentation}");
  });
});
