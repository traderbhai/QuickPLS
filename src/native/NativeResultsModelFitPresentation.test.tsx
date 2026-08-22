import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { ResultTable } from "../domain/resultTables";
import NativeResultsSurface from "./NativeResultsSurface";
import {
  nativePlsModelFitPresentationFixture,
  type NativePlsModelFitPresentationFixture,
} from "./nativePlsModelFitPresentation.testFixtures";
import {
  nativeModelFitPresentationStateV2,
  type NativeResultNavigation,
} from "./nativeResults";

const navigation: NativeResultNavigation = {
  runId: "model-fit-presentation",
  defaultItemId: "path_coefficients",
  groups: [{
    id: "final_results",
    title: "Results",
    items: [{
      id: "path_coefficients",
      kind: "table",
      title: "Path coefficients",
      tableId: "path_coefficients",
    }],
  }],
  tables: [],
};

const selectedTable: ResultTable = {
  id: "path_coefficients",
  title: "Path coefficients",
  status: "validated",
  warning: null,
  columns: ["Path", "Estimate"],
  rows: [["X → Y", "0.5000"]],
};

describe("native Results exact-fit presentation contract", () => {
  const expected: Array<{
    fixture: NativePlsModelFitPresentationFixture;
    mode: string;
    label: string;
  }> = [
    { fixture: "not_run", mode: "descriptive", label: "Not run" },
    { fixture: "available", mode: "exact_available", label: "Available" },
    { fixture: "partial", mode: "exact_partial", label: "Partial" },
    { fixture: "unavailable", mode: "exact_unavailable", label: "Unavailable" },
    { fixture: "failed", mode: "exact_failed", label: "Failed" },
  ];

  for (const row of expected) {
    it(`renders exact-fit presentation state ${row.label} from validated run authority`, () => {
      const run = nativePlsModelFitPresentationFixture(row.fixture);
      expect(nativeModelFitPresentationStateV2(run)).toMatchObject({
        mode: row.mode,
        detailValue: row.label,
      });
      const markup = renderToStaticMarkup(<NativeResultsSurface
        runs={[run]}
        selectedRun={run}
        selectedRunId={run.id}
        setSelectedRunId={vi.fn()}
        navigation={{ ...navigation, runId: run.id }}
        selectedItem={navigation.groups[0].items[0]}
        selectedTable={selectedTable}
        setSelectedTableId={vi.fn()}
        propertiesOpen
      />);
      expect(markup).toContain(`Exact-fit bootstrap</dt><dd>${row.label}</dd>`);
    });
  }
});
