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
      "permutation parameter table",
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

    expect(reports).toContain("Run comparison table");
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
  it("keeps SEM canvas overlay state visible to users", () => {
    const canvas = read("src/components/ModelCanvas.tsx");
    const latent = read("src/components/LatentNode.tsx");
    const styles = read("src/styles.css");

    expect(canvas).toContain("canvas-overlay-status");
    expect(canvas).toContain("canvas-next-action");
    expect(canvas).toContain("Model-only diagram");
    expect(canvas).toContain("Result overlay active");
    expect(canvas).toContain("Overlay blocked");
    expect(canvas).toContain("Recommended next workflow action");
    expect(canvas).toContain("R²");
    expect(latent).toContain("R²");
    expect(canvas).not.toContain(mojibakeR2);
    expect(latent).not.toContain(mojibakeR2);
    expect(styles).toContain(".canvas-overlay-status");
    expect(styles).toContain(".canvas-next-action");
    expect(styles).toContain(".canvas-overlay-status.ready");
    expect(styles).toContain(".canvas-overlay-status.warning");
  });

  it("keeps documented SEM canvas keyboard shortcuts wired", () => {
    const canvas = read("src/components/ModelCanvas.tsx");

    for (const shortcut of [
      'event.key.toLowerCase() === "z"',
      'event.key.toLowerCase() === "y"',
      'event.key.toLowerCase() === "d"',
      'event.key === "Delete"',
      'event.key === "Backspace"',
      'event.key === "Escape"',
      'event.key === "Enter"',
      'event.key.toLowerCase() === "p"',
      'event.key.toLowerCase() === "c"',
      'event.key.toLowerCase() === "v"',
      'event.key.toLowerCase() === "f"',
    ]) {
      expect(canvas).toContain(shortcut);
    }

    expect(canvas).toContain('window.prompt("Construct name"');
    expect(canvas).toContain('window.prompt("Path label"');
    expect(canvas).toContain("if (isEditingText(event.target)) return;");
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

