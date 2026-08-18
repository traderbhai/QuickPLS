import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { CanonicalResultComparisonDocumentV2 } from "../domain/canonicalResultComparisonV2";
import { CanonicalComparisonOutcomeV2 } from "./RunHistory";
import {
  canonicalComparisonAvailabilityCopyV2,
  type CanonicalRunComparisonUiStateV2,
} from "./canonicalRunComparisonUiV2";

const capabilityCell = {
  registry_schema_version: 2 as const,
  capability_id: "smartpls.pls_algorithm",
  cell_id: "qpls3.pls.algorithm",
  capability_version: "1.0.0",
};

function readyComparison(): CanonicalResultComparisonDocumentV2 {
  return {
    schema_version: 2,
    comparison_id: "comparison:first:to:second",
    title: "First compared with Second",
    sources: { left_document_id: "result:first", right_document_id: "result:second" },
    analytical_identity: {
      capability_cell: capabilityCell,
      capability_cells: [capabilityCell],
      dataset_fingerprint: "a".repeat(64),
      model_digest: "b".repeat(64),
      recipe_digest: "c".repeat(64),
    },
    tables: [{
      id: "comparison:path_coefficients",
      source_table_id: "path_coefficients",
      title: "Path coefficients",
      capability_cells: [capabilityCell],
      columns: [{ id: "coefficient", label: "Coefficient", data_type: "number", description: "Path coefficient." }],
      rows: [{
        id: "row_1",
        source_row_id: "row_1",
        changed_cell_count: 1,
        cells: [{
          id: "row_1:coefficient",
          column_id: "coefficient",
          kind: "number",
          left: 0.4,
          right: 0.5,
          change: 0.1,
          absolute_change: 0.1,
          changed: true,
        }],
      }],
      changed_cell_count: 1,
    }],
    summary: { table_count: 1, row_count: 1, cell_count: 1, changed_cell_count: 1 },
  };
}

describe("canonical run comparison UI v2", () => {
  it("shows loading feedback without rendering side-by-side values", () => {
    const markup = renderToStaticMarkup(<CanonicalComparisonOutcomeV2 state={{ status: "loading" }} firstName="First" secondName="Second" />);
    expect(markup).toContain("Checking comparison");
    expect(markup).toContain('role="status"');
    expect(markup).not.toContain("Change is the second selected run");
  });

  it("shows the exact compatibility title and corrective message, with no deltas", () => {
    const state: CanonicalRunComparisonUiStateV2 = {
      status: "blocked",
      issues: [{
        id: "dataset_mismatch",
        code: "dataset_mismatch",
        title: "Data differs",
        message: "These results use different data. Choose runs calculated from the same dataset.",
        related_ids: [],
        technical_details: [],
      }],
    };
    const markup = renderToStaticMarkup(<CanonicalComparisonOutcomeV2 state={state} firstName="First" secondName="Second" />);
    expect(markup).toContain("Data differs");
    expect(markup).toContain("These results use different data. Choose runs calculated from the same dataset.");
    expect(markup).toContain('role="alert"');
    expect(markup).not.toContain("Path coefficients side by side");
    expect(canonicalComparisonAvailabilityCopyV2(state)).toEqual({
      available: false,
      description: "Data differs: These results use different data. Choose runs calculated from the same dataset.",
      actionTitle: "These results use different data. Choose runs calculated from the same dataset.",
    });
  });

  it("renders canonical typed rows only for a ready comparison", () => {
    const state: CanonicalRunComparisonUiStateV2 = { status: "ready", comparison: readyComparison() };
    const markup = renderToStaticMarkup(<CanonicalComparisonOutcomeV2 state={state} firstName="First run" secondName="Second run" />);
    expect(markup).toContain("Exact comparison ready");
    expect(markup).toContain("Path coefficients side by side");
    expect(markup).toContain("First run");
    expect(markup).toContain("Second run");
    expect(markup).toContain("0.4");
    expect(markup).toContain("0.5");
    expect(markup).toContain("0.1");
    expect(canonicalComparisonAvailabilityCopyV2(state)).toMatchObject({
      available: true,
      description: "1 typed result row(s) are comparable in Results; 1 value(s) differ.",
    });
  });
});
