import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const read = (path: string) => readFileSync(path, "utf8");
const mojibakeR2 = `R${String.fromCharCode(0x00c2)}²`;
const doubleEncodedR2 = `R${String.fromCharCode(0x00c3)}${String.fromCharCode(0x201a)}${String.fromCharCode(0x00c2)}²`;

describe("desktop accessibility contracts", () => {
  it("keeps large table surfaces keyboard-focusable and named", () => {
    const data = read("src/components/DataWorkspace.tsx");
    const results = read("src/components/RunHistory.tsx");
    const groups = read("src/components/GroupsWorkspace.tsx");
    const reports = read("src/components/ReportsWorkspace.tsx");

    expect(data).toContain('className="data-grid" tabIndex={0} role="region"');
    expect(data).toContain("Data preview table for");

    for (const label of [
      "result summary",
      "measurement quality tables",
      "bootstrap parameter table",
      "Structural path randomization",
    ]) {
      expect(results).toContain(label);
    }

    for (const label of [
      "Two-group MGA comparisons table",
      "Permutation MGA comparisons table",
      "MICOM invariance table",
      "FIMIX class paths table",
      "PLS-POS segment paths table",
      "IPMA importance performance table",
    ]) {
      expect(groups).toContain(label);
    }

    expect(reports).toContain("report-comparison-link");
    expect(reports).toContain("Open Results Comparison");
    expect(reports).toContain('aria-label={`${table.title} table`}');
  });

  it("uses visible table focus treatment and avoids R-squared mojibake", () => {
    const styles = read("src/styles.css");
    const reports = read("src/components/ReportsWorkspace.tsx");

    expect(styles).toContain(".data-grid:focus-visible");
    expect(styles).toContain(".bootstrap-table-scroll:focus-visible");
    expect(styles).toContain(".result-summary:focus-visible");
    expect(styles).toContain(".quality-summary:focus-visible");

    expect(reports).toContain("R<sup>2</sup>");
    expect(reports).not.toContain(doubleEncodedR2);
    expect(reports).not.toContain("Rï¿½");
  });
  it("keeps model commands in the native shell instead of duplicating them inside the canvas", () => {
    const canvas = read("src/components/ModelCanvas.tsx");
    const native = read("src/native/NativeDesktopApp.tsx");
    const inspector = read("src/native/NativeModelInspector.tsx");
    const canvasStyles = read("src/native/nativeCanvas.css");
    const latent = read("src/components/LatentNode.tsx");

    expect(native).toContain('className="nd-commandbar" role="toolbar"');
    expect(canvas).not.toContain('aria-label="Model editing tools"');
    expect(canvas).not.toContain('className="canvas-toolbar');
    expect(canvas).not.toContain("CFA measurement preset");
    expect(canvas).not.toContain("Mediation preset");
    expect(canvas).not.toContain("Arrange like SmartPLS");
    expect(canvas).toContain('window.addEventListener("quickpls:model-tool"');
    expect(canvas).toContain('window.addEventListener("quickpls:model-arrange"');
    expect(canvas).toContain('window.addEventListener("quickpls:model-fit"');
    expect(canvas).toContain("<Controls showInteractive={false} />");
    expect(canvas).not.toContain('className="diagram-context-menu"');
    expect(canvas).not.toContain("window.prompt(");
    expect(native).toContain('case "model.edit-selection"');
    expect(inspector).toContain('id="nd-model-construct-name"');
    expect(inspector).toContain('id="nd-model-path-label"');
    expect(canvasStyles).not.toContain(".canvas-toolbar");
    expect(canvasStyles).not.toContain(".diagram-help");
    expect(canvasStyles).not.toContain(".diagram-context-menu");
    expect(latent).toContain("R²");
    expect(latent).not.toContain(mojibakeR2);
  });

  it("keeps a persistent desktop readiness checklist in the status bar", () => {
    const statusBar = read("src/components/StatusBar.tsx");
    const styles = read("src/styles.css");

    expect(statusBar).toContain('aria-label="Persistent analysis readiness checklist"');
    expect(statusBar).toContain("readiness.items.map");
    expect(statusBar).toContain("status-readiness-pill");
    expect(statusBar).toContain("item.detail");
    expect(styles).toContain(".status-readiness-strip");
    expect(styles).toContain(".status-readiness-pill.ready");
    expect(styles).toContain(".status-readiness-pill.warning");
    expect(styles).toContain(".status-readiness-pill.blocked");
  });
});
