import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { Dataset } from "../types";
import { NativeDatasetTransformDialog } from "./NativeDatasetTransformDialog";

const dataset: Dataset = {
  id: "dataset-source",
  name: "Survey",
  columns: ["score", "other"],
  rows: [{ score: 1, other: 2 }],
  rowCount: 1,
  missing: 0,
  fingerprint: "sha256:source",
  kind: "raw",
  columnMetadata: [
    { name: "score", label: null, column_type: "numeric", scale_type: "ordinal", missing_markers: [], theoretical_min: 1, theoretical_max: 5, value_labels: {} },
    { name: "other", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
  ],
};

describe("NativeDatasetTransformDialog", () => {
  it("renders an accessible preview-before-commit workflow for every operation", () => {
    const markup = renderToStaticMarkup(<NativeDatasetTransformDialog
      dataset={dataset}
      selectedColumn="score"
      nativeDesktop
      projectWritable
      mutationsLocked={false}
      datasetResident
      dialogScope={1}
      close={vi.fn()}
      complete={vi.fn()}
      onBusyChange={vi.fn()}
      previewTransformation={vi.fn()}
      applyTransformation={vi.fn()}
    />);

    expect(markup).toContain("Reverse scale");
    expect(markup).toContain("Recode values");
    expect(markup).toContain("Arithmetic");
    expect(markup).toContain("Row sum or mean");
    expect(markup).toContain("Dummy variable");
    expect(markup).toContain("Group variable");
    expect(markup).toContain("Preview does not change the dataset");
    expect(markup).toContain("Preview required");
    expect(markup).toContain(">Preview</button>");
    expect(markup).toMatch(/<button[^>]*disabled=""[^>]*>Create Version<\/button>/);
    expect(markup).toContain("New dataset version name");
    expect(markup).toContain("Output scale");
  });

  it("renders the active-calculation block in the dialog", () => {
    const markup = renderToStaticMarkup(<NativeDatasetTransformDialog
      dataset={dataset}
      selectedColumn="score"
      nativeDesktop
      projectWritable
      mutationsLocked
      datasetResident
      dialogScope={2}
      close={vi.fn()}
      complete={vi.fn()}
      onBusyChange={vi.fn()}
      previewTransformation={vi.fn()}
      applyTransformation={vi.fn()}
    />);

    expect(markup).toContain("Finish or cancel the active calculation before deriving a variable.");
    expect(markup).toContain('role="status"');
  });
});
