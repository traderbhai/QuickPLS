import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { Dataset } from "../types";
import {
  NativeSemParameterTable,
  observedSemanticsForParameterTable,
  semDataBindingForParameterTable,
} from "./NativeSemParameterTable";

function dataset(patch: Partial<Dataset> = {}): Dataset {
  return {
    id: "data-a",
    name: "Data A",
    columns: ["x1", "x2", "unused"],
    rows: [],
    missing: 0,
    ...patch,
  };
}

describe("native SEM parameter table inputs", () => {
  it("renders Advanced Parameter Table as a labelled, focusable modal region", () => {
    const html = renderToStaticMarkup(createElement(NativeSemParameterTable, {
      modelName: "Current model",
      presentation: "dialog",
      onShowCanvas: () => undefined,
      onContinueToCalculation: () => undefined,
    }));

    expect(html).toContain('id="nd-advanced-parameter-table"');
    expect(html).toContain('role="region"');
    expect(html).toContain('aria-labelledby="nd-advanced-parameter-table-heading"');
    expect(html).toContain('tabindex="0"');
    expect(html).toContain("Advanced Parameter Table");
    expect(html).toContain("Continue to Calculate");
    expect(html).toContain('aria-label="Scrollable parameter table"');
  });

  it("uses the current raw dataset identity and explicit listwise policy", () => {
    expect(semDataBindingForParameterTable(dataset())).toEqual({
      kind: "raw",
      dataset_id: "data-a",
      missing_data: "listwise_deletion",
      weight: null,
      cluster_variable: null,
      strata_variable: null,
    });
  });

  it("does not invent a matrix sample size when metadata is absent", () => {
    expect(semDataBindingForParameterTable(dataset({ kind: "covariance", sampleSize: null }))).toEqual({
      kind: "covariance",
      dataset_id: "data-a",
      variables: ["x1", "x2", "unused"],
      means: null,
      standard_deviations: null,
      sample: { sample_size: 0, covariance_denominator: "sample_n_minus_one" },
    });
  });

  it("projects only used indicator metadata in deterministic order", () => {
    const semantics = observedSemanticsForParameterTable(dataset({
      columnMetadata: [
        { name: "unused", label: null, column_type: "text", scale_type: "nominal", missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {} },
        { name: "x2", label: "Second item", column_type: "numeric", scale_type: "ordinal", missing_markers: ["99"], theoretical_min: 1, theoretical_max: 5, value_labels: { "2": "Two", "1": "One" } },
      ],
    }), ["x2", "x1", "x2"]);

    expect(Object.keys(semantics)).toEqual(["x2"]);
    expect(semantics.x1).toBeUndefined();
    expect(semantics.x2).toEqual({
      label: "Second item",
      scale: "ordinal",
      role: "indicator",
      categories: ["1", "2"],
      value_labels: { "1": "One", "2": "Two" },
      missing_markers: ["99"],
      transformation_lineage: [],
    });
  });

  it("canonicalizes import marker provenance before SemModelV4 validation", () => {
    const semantics = observedSemanticsForParameterTable(dataset({
      columnMetadata: [
        { name: "x1", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: ["", " NA ", "NA", ".", "  ", "N/A", "."], theoretical_min: null, theoretical_max: null, value_labels: {} },
      ],
    }), ["x1"]);

    expect(semantics.x1?.missing_markers).toEqual([".", "N/A", "NA"]);
  });

  it("sorts import marker provenance by UTF-8 bytes", () => {
    const semantics = observedSemanticsForParameterTable(dataset({
      columnMetadata: [
        { name: "x1", label: null, column_type: "numeric", scale_type: "continuous", missing_markers: ["\u{10000}", "\uE000"], theoretical_min: null, theoretical_max: null, value_labels: {} },
      ],
    }), ["x1"]);

    expect(semantics.x1?.missing_markers).toEqual(["\uE000", "\u{10000}"]);
  });

  it("uses the canonical strict model and emits one atomic parameter or variable intent", () => {
    const source = readFileSync("src/native/NativeSemParameterTable.tsx", "utf8");
    expect(source).toContain("projectSemModelV4ParameterTable(strictAuthority.model, strictTrace)");
    expect(source).toContain("commitStandardSemModelV4Intent");
    expect(source).toContain("if (intents.length !== 1)");
    for (const kind of ["set_parameter_specification", "restore_parameter", "set_factor_identification", "set_latent_mean", "set_observed_intercept", "set_ordinal_thresholds"]) {
      expect(source).toContain(`kind: \"${kind}\"`);
    }
  });
});
