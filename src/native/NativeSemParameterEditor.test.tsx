import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import type { SemParameterV4, SemVariableV4 } from "../domain/semModelV4";
import { NativeSemParameterEditor, NativeSemVariableEditor } from "./NativeSemParameterEditor";

const loading: Exclude<SemParameterV4, { kind: "derived" }> = {
  kind: "free",
  id: "loading-x2",
  label: "X to x2",
  target: { kind: "loading", construct: "construct:x", indicator: "observed:x2" },
  start: 0.7,
  lower: 0,
  upper: 1,
  equality_label: "loading_a",
  group_overrides: [],
};

describe("Experimental SemModelV4 parameter editors", () => {
  it("renders labelled free/fixed, start, bound, and equality controls without contenteditable cells", () => {
    const html = renderToStaticMarkup(<NativeSemParameterEditor
      parameter={loading}
      canRestore
      onApply={vi.fn()}
      onRestore={vi.fn()}
      onClose={vi.fn()}
    />);

    expect(html).toContain("Specification");
    expect(html).toContain("Start value");
    expect(html).toContain("Lower bound");
    expect(html).toContain("Upper bound");
    expect(html).toContain("Equality label");
    expect(html).toContain("Restore generated setting");
    expect(html).not.toContain("contenteditable");
  });

  it("shows all common-factor identification choices and the latent-mean switch", () => {
    const variable: Extract<SemVariableV4, { kind: "common_factor" }> = {
      kind: "common_factor",
      id: "construct:x",
      label: "Predictor",
      identification: { kind: "marker_loading", indicator: "observed:x1" },
      mean_policy: { kind: "fixed_zero" },
      disturbance_policy: { kind: "exogenous_variance", parameter: "variance-x" },
    };
    const html = renderToStaticMarkup(<NativeSemVariableEditor
      variable={variable}
      indicators={["x1", "x2", "x3"]}
      hasLatentMean={false}
      hasIntercept={false}
      hasThresholds={false}
      onApply={vi.fn()}
      onClose={vi.fn()}
    />);

    expect(html).toContain("Factor identification");
    expect(html).toContain("Marker loading");
    expect(html).toContain("Fixed variance");
    expect(html).toContain("Effects coding");
    expect(html).toContain("Estimate latent mean");
  });

  it("offers thresholds only through ordinal variable metadata", () => {
    const variable: Extract<SemVariableV4, { kind: "observed" }> = {
      kind: "observed",
      id: "observed:x3",
      label: "Ordered item",
      source_column: "x3",
      scale: "ordinal",
      role: "indicator",
      categories: ["low", "middle", "high"],
      value_labels: {},
      missing_markers: [],
      transformation_lineage: [],
    };
    const html = renderToStaticMarkup(<NativeSemVariableEditor
      variable={variable}
      indicators={["x3"]}
      hasLatentMean={false}
      hasIntercept={false}
      hasThresholds
      onApply={vi.fn()}
      onClose={vi.fn()}
    />);

    expect(html).toContain("Estimate ordinal thresholds");
    expect(html).toMatch(/Estimate observed intercept/);
    expect(html).toMatch(/disabled=""[^>]*\/>Estimate observed intercept|disabled=""/);
  });
});
