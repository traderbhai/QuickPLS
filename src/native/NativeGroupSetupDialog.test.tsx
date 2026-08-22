import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useWorkspace } from "../store";
import type { Dataset } from "../types";
import NativeGroupSetupDialog, { nativeInitialGroupingColumn } from "./NativeGroupSetupDialog";

const dataset: Dataset = {
  id: "group-data",
  name: "groups.csv",
  columns: ["x", "y", "segment"],
  rows: Array.from({ length: 24 }, (_, index) => ({
    x: index + 1,
    y: index * 2 + 1,
    segment: index < 12 ? "A" : "B",
  })),
  rowCount: 24,
  missing: 0,
  kind: "raw",
  columnMetadata: [{
    name: "segment",
    label: "Customer segment",
    column_type: "text",
    scale_type: "nominal",
    missing_markers: [],
    theoretical_min: null,
    theoretical_max: null,
    value_labels: { A: "Established", B: "New" },
  }],
};

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("NativeGroupSetupDialog", () => {
  it("prefers an eligible requested variable and falls back to the saved group variable", () => {
    expect(nativeInitialGroupingColumn(dataset, ["x", "y"], "segment", null)).toBe("segment");
    expect(nativeInitialGroupingColumn(dataset, ["x", "y"], "x", "segment")).toBe("segment");
    expect(nativeInitialGroupingColumn(dataset, ["x", "y", "segment"], "segment", "segment")).toBe("");
  });

  it("renders explicit A and B selection with full-data counts and the bounded scientific scope", () => {
    vi.stubGlobal("window", {});
    const settings = {
      ...useWorkspace.getState().analysisSettings,
      groupColumn: "segment",
      groupAValue: "A",
      groupBValue: "B",
    };
    const markup = renderToStaticMarkup(<NativeGroupSetupDialog
      dataset={dataset}
      analysisColumns={["x", "y"]}
      initialColumn="segment"
      settings={settings}
      nativeDesktop={false}
      projectWritable
      apply={vi.fn()}
      close={vi.fn()}
    />);

    expect(markup).toContain("Grouping variable");
    expect(markup).toContain("Customer segment [segment]");
    expect(markup).toContain("Group A");
    expect(markup).toContain("Group B");
    expect(markup).toContain("Established [A]");
    expect(markup.match(/<td>12<\/td>/g)).toHaveLength(4);
    expect(markup).toContain("Complete model cases");
    expect(markup).not.toContain("Group A − Group B");
    expect(markup).not.toContain("combined MICOM and structural-path permutation MGA workflow");
    expect(markup).not.toContain("Step 1 confirmation");
    expect(markup).not.toContain("shared permutation plan");
    expect(markup).not.toContain('<option value="x"');
    expect(markup).not.toContain('<option value="y"');
    expect(markup).toMatch(/type="submit">Apply Groups/);
  });

  it("blocks Apply when either selected group has fewer than ten complete model cases", () => {
    vi.stubGlobal("window", {});
    const small: Dataset = {
      ...dataset,
      id: "small-groups",
      rows: dataset.rows.slice(0, 15).map((row, index) => ({ ...row, segment: index < 8 ? "A" : "B" })),
      rowCount: 15,
    };
    const settings = {
      ...useWorkspace.getState().analysisSettings,
      groupColumn: "segment",
      groupAValue: "A",
      groupBValue: "B",
    };
    const markup = renderToStaticMarkup(<NativeGroupSetupDialog
      dataset={small}
      analysisColumns={["x", "y"]}
      initialColumn="segment"
      settings={settings}
      nativeDesktop={false}
      projectWritable
      apply={vi.fn()}
      close={vi.fn()}
    />);

    expect(markup).toContain("at least 10 are required");
    expect(markup).toMatch(/class="primary" type="submit" disabled=""/);
  });
});
