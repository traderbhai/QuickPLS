import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { ResultTable } from "../domain/resultTables";
import { NativeResultTable } from "./NativeResultTable";

const table: ResultTable = {
  id: "moderation_effects",
  title: "Moderating effects",
  status: "validated",
  warning: null,
  columns: ["Moderating effect", "Estimate", "Bootstrap mean", "P value", "Lower", "Upper"],
  rows: [["Satisfaction × Trust → Loyalty", "0.2200", "0.2180", "0.0100", "0.0800", "0.3500"]],
  presentation: { rows: [{ key: "moderation:0", nodeIds: ["satisfaction", "trust", "loyalty"] }] },
};

describe("NativeResultTable", () => {
  it("renders semantic alignment, sticky identity, responsive priority, confidence and overflow hooks", () => {
    const html = renderToStaticMarkup(<NativeResultTable
      table={table}
      gridKey="run:table"
      headingId="table-heading"
      confidenceLevel={0.95}
    />);

    expect(html).toContain('data-result-horizontal-scroll="true"');
    expect(html).toContain('data-result-overflow-risk="true"');
    expect(html).toContain('data-result-identity-column="true"');
    expect(html).toContain('data-result-column-kind="number"');
    expect(html).toContain('data-result-column-priority="tertiary"');
    expect(html).toContain('data-result-row-model-focus="true"');
    expect(html).toContain("95% confidence intervals");
    expect(html).toContain("More columns are available horizontally.");
  });

  it("retains the native scientific grid keyboard and copy contract", () => {
    const html = renderToStaticMarkup(<NativeResultTable table={table} gridKey="run:table" headingId="table-heading" />);

    expect(html).toContain('role="grid"');
    expect(html).toContain('aria-keyshortcuts="Control+C"');
    expect(html).toContain('data-native-grid-cell="true"');
    expect(html).toContain("Use the arrow keys to move between cells.");
  });
});
