import { describe, expect, it } from "vitest";
import type { ResultTable } from "./resultTables";
import { canonicalResultTablePresentation, resultTablePresentation } from "./resultTablePresentation";

function table(overrides: Partial<ResultTable> = {}): ResultTable {
  return {
    id: "paths",
    title: "Path coefficients",
    status: "validated",
    warning: null,
    columns: ["Relationship", "Estimate", "Bootstrap mean", "P value", "Lower", "Upper"],
    rows: [["Satisfaction → Loyalty", "0.4100", "0.4060", "0.0020", "0.2500", "0.5600"]],
    ...overrides,
  };
}

describe("result table presentation", () => {
  it("marks one authored identity column as sticky and aligns scientific values numerically", () => {
    const presentation = resultTablePresentation(table(), 0.95);

    expect(presentation.identityColumnIndex).toBe(0);
    expect(presentation.columns[0]).toMatchObject({ kind: "identity", priority: "primary", sticky: true });
    expect(presentation.columns[1]).toMatchObject({ kind: "number", priority: "secondary", sticky: false });
    expect(presentation.columns[2]).toMatchObject({ kind: "number", priority: "tertiary" });
    expect(presentation.confidenceHeading).toBe("95% confidence intervals");
    expect(presentation.hasHorizontalOverflowRisk).toBe(true);
  });

  it("finds the first textual identity when a legacy table has no recognized heading", () => {
    const presentation = resultTablePresentation(table({
      columns: ["Scenario", "N", "Power"],
      rows: [["Conservative", "120", "0.802"]],
    }));

    expect(presentation.identityColumnIndex).toBe(0);
    expect(presentation.columns.map((column) => column.kind)).toEqual(["identity", "number", "number"]);
  });

  it("does not invent an identity or confidence heading for an all-numeric table", () => {
    const presentation = resultTablePresentation(table({
      columns: ["N", "Estimate"],
      rows: [["100", "0.2"]],
    }));

    expect(presentation.identityColumnIndex).toBeNull();
    expect(presentation.columns.every((column) => column.kind === "number")).toBe(true);
    expect(presentation.confidenceHeading).toBeNull();
  });

  it("uses canonical column roles and data types without inspecting formatted values", () => {
    const presentation = canonicalResultTablePresentation({
      id: "effects",
      title: "Effects",
      columns: [
        { id: "effect", label: "Effect", data_type: "text", description: "Authored effect.", role: "label" },
        { id: "estimate", label: "Estimate", data_type: "number", description: "Estimate.", role: "estimate" },
        { id: "lower", label: "95% CI lower", data_type: "number", description: "Lower bound.", role: "uncertainty" },
      ],
      rows: [],
      footnote_ids: [],
    });

    expect(presentation.columns).toEqual([
      expect.objectContaining({ kind: "identity", priority: "primary", sticky: true }),
      expect.objectContaining({ kind: "number", priority: "secondary" }),
      expect.objectContaining({ kind: "number", priority: "tertiary" }),
    ]);
    expect(presentation.confidenceHeading).toBe("95% confidence intervals");
  });

  it("honors explicit legacy-table presentation hints over heuristics", () => {
    const presentation = resultTablePresentation(table({
      columns: ["Saved name", "Reported value"],
      rows: [["Model A", "not estimated"]],
      presentation: {
        columns: [
          { kind: "identity", priority: "primary", sticky: true },
          { kind: "number", priority: "tertiary" },
        ],
        confidenceLevel: 0.9,
      },
    }));

    expect(presentation.columns[0]).toMatchObject({ kind: "identity", sticky: true });
    expect(presentation.columns[1]).toMatchObject({ kind: "number", priority: "tertiary" });
  });
});
