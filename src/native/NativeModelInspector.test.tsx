import { renderToStaticMarkup } from "react-dom/server";
import type { Node } from "@xyflow/react";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useWorkspace } from "../store";
import { convertLegacyBasicModelV4 } from "../domain/semModelV4";
import {
  NativeModelInspector,
  nativeModelInspectorPreflightPreview,
  nextNativeModelInspectorTab,
} from "./NativeModelInspector";
import type { NativePlsReadiness } from "./nativePlsReadiness";
import type { ConstructData } from "../types";

const readyReadiness: NativePlsReadiness = {
  canRun: true,
  summary: "Ready to calculate",
  blockers: [],
  warnings: [],
  items: [
    { id: "runtime", label: "Runtime", detail: "QuickPLS desktop runtime is available.", status: "ready" },
    { id: "calculation", label: "Calculation", detail: "PLS-SEM Algorithm estimation is selected.", status: "ready" },
  ],
};

function projectedNode(data: Partial<ConstructData>): Node<ConstructData> {
  const base = useWorkspace.getState().nodes[0];
  if (!base) throw new Error("Expected the reset project to contain a construct.");
  const node = {
    ...base,
    id: "projected-truth-node",
    data: {
      ...base.data,
      semantic: undefined,
      interaction: undefined,
      higherOrder: undefined,
      ...data,
    },
  };
  return node;
}

describe("native model inspector customer workflow", () => {
  beforeEach(() => {
    useWorkspace.getState().resetProject();
    useWorkspace.getState().setUiPreferences({ experimentalLabsEnabled: false });
    const firstNode = useWorkspace.getState().nodes[0];
    useWorkspace.getState().setSelectedNode(firstNode?.id ?? null);
  });

  afterEach(() => useWorkspace.getState().setUiPreferences({ experimentalLabsEnabled: false }));

  it("organizes the live editor into four named keyboard tabs with Basic as the safe default", () => {
    const html = renderToStaticMarkup(<NativeModelInspector />);

    expect(html).toContain('role="group" aria-label="Inspector mode"');
    expect(html).toContain('aria-pressed="true">Basic</button>');
    expect(html).toContain('role="tablist" aria-label="Model inspector sections"');
    for (const label of ["Model", "Parameter", "Appearance", "Data Binding"]) {
      expect(html).toContain(`>${label}</button>`);
    }
    expect(html).toContain('role="tabpanel"');
    expect(html).toContain('id="nd-model-construct-name"');
    expect(html).not.toContain("Scientific representation");
  });

  it("reveals scientific parameter controls only in Expert mode when Experimental Labs is enabled", () => {
    useWorkspace.getState().setUiPreferences({ experimentalLabsEnabled: true });

    const html = renderToStaticMarkup(<NativeModelInspector
      initialMode="expert"
      initialTab="parameter"
      experimentalLabsEnabledOverride
    />);

    expect(html).toContain('aria-pressed="true">Expert</button>');
    expect(html).toContain('aria-selected="true" aria-controls="nd-model-inspector-parameter-panel"');
    expect(html).toContain("Measurement model");
    expect(html).toContain("Scientific representation");
    expect(html).not.toContain("Expert mode: scientific and detailed controls.");
  });

  it("keeps Expert mode useful in Standard builds without exposing Labs authoring", () => {
    const html = renderToStaticMarkup(<NativeModelInspector initialMode="expert" initialTab="parameter" />);

    expect(html).toContain("Stable construct ID");
    expect(html).toContain("Bound indicators");
    expect(html).not.toContain("Scientific authoring remains in Experimental Labs.");
    expect(html).not.toContain("Scientific representation");
  });

  it("exposes the complete canonical document fallback only for Expert strict Standard models", () => {
    const state = useWorkspace.getState();
    const modelId = state.activeModelId;
    if (!modelId) throw new Error("Expected an active model after reset.");
    const model = convertLegacyBasicModelV4({
      id: modelId,
      name: "Strict Standard model",
      constructs: [
        { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
        { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
      ],
      paths: [{ source: "x", target: "y" }],
      controls: [],
      higher_order_constructs: [],
      interactions: [],
    }, "cbsem_common_factor");
    expect(state.installStandardSemModelV4Authority({
      schema_version: 1,
      model_document_sha256: "a".repeat(64),
      model,
    })).toBe(true);

    const authority = useWorkspace.getState().standardSemModelV4Authorities[modelId];
    const basic = renderToStaticMarkup(<NativeModelInspector initialMode="basic" initialTab="model" strictAuthorityOverride={authority} />);
    const expert = renderToStaticMarkup(<NativeModelInspector initialMode="expert" initialTab="model" strictAuthorityOverride={authority} />);

    expect(basic).not.toContain("Advanced canonical document");
    expect(expert).toContain("Advanced canonical document");
    expect(expert).not.toContain("strict Standard scientific controls, including the complete canonical document fallback");
  });

  it("renders a plain-language Basic preflight and preserves the real calculation plan verbatim", () => {
    const html = renderToStaticMarkup(<NativeModelInspector readiness={readyReadiness} />);

    expect(html).toContain("Model preflight");
    expect(html).toContain('role="status" aria-live="polite" aria-atomic="true"');
    expect(html).toContain("Ready to calculate");
    expect(html).toContain("All required model, data, runtime, and calculation checks passed.");
    expect(html).toContain("Workload preview");
    expect(html).toContain("PLS-SEM Algorithm estimation is selected.");
    expect(html).toContain("Assigned indicators");
    expect(html).not.toContain("All preflight checks");
  });

  it("shows every real blocker in Expert mode without inventing a runtime or duration estimate", () => {
    const blockers = [
      { id: "runtime", label: "Runtime", detail: "Calculations require the offline QuickPLS desktop runtime.", status: "blocked" as const },
      { id: "indicators", label: "Indicators", detail: "Satisfaction needs at least one assigned indicator.", status: "blocked" as const },
    ];
    const readiness: NativePlsReadiness = {
      canRun: false,
      summary: "2 blockers before calculation",
      blockers,
      warnings: [],
      items: [...blockers, { id: "calculation", label: "Calculation", detail: "PLS-SEM Algorithm estimation is selected.", status: "ready" }],
    };
    const html = renderToStaticMarkup(<NativeModelInspector initialMode="expert" readiness={readiness} />);

    expect(html).toContain("2 things to fix before calculation");
    expect(html).toContain("Calculations require the offline QuickPLS desktop runtime.");
    expect(html).toContain("All preflight checks (3)");
    expect(html).toContain("Runtime: Fix required");
    expect(html).toContain("Indicators: Fix required");
    expect(html).not.toMatch(/\b(minutes?|seconds?|hours?)\b/i);
  });

  it("maps a real warning to review language while retaining the calculation detail", () => {
    const warning = { id: "data", label: "Data", detail: "The complete file is checked again before calculation.", status: "warning" as const };
    const preview = nativeModelInspectorPreflightPreview({
      canRun: true,
      summary: "Ready with 1 warning",
      blockers: [],
      warnings: [warning],
      items: [warning, readyReadiness.items[1]],
    });

    expect(preview).toEqual({
      status: "warning",
      headline: "Ready, with 1 item to review",
      detail: warning.detail,
      calculationPlan: "PLS-SEM Algorithm estimation is selected.",
    });
  });

  it("provides real, named data-binding controls without relying on canvas drag", () => {
    const selected = useWorkspace.getState().nodes[0];
    const html = renderToStaticMarkup(<NativeModelInspector
      initialTab="data-binding"
      selectedNodeIdOverride={selected.id}
      selectedEdgeIdOverride={null}
    />);

    expect(html).toContain("Assign dataset variable");
    expect(html).toContain(`aria-label="Variables bound to ${selected.data.label}"`);
    expect(html).toContain(`aria-label="Remove ${selected.data.indicators[0]} from ${selected.data.label}"`);
    expect(html).toContain("Choose variable…");
  });

  it("keeps relationship routing in Appearance and explains its non-scientific scope", () => {
    const relationship = useWorkspace.getState().edges.find((edge) => !edge.id.startsWith("measurement::"));
    expect(relationship).toBeDefined();
    useWorkspace.getState().setSelectedEdge(relationship!.id);

    const html = renderToStaticMarkup(<NativeModelInspector
      initialTab="appearance"
      selectedNodeIdOverride={null}
      selectedEdgeIdOverride={relationship!.id}
    />);

    expect(html).toContain("Routing");
    expect(html).toContain("Routing changes presentation only; it does not change the scientific relationship.");
  });

  it("renders every projected interaction construction from canonicalMethod exactly", () => {
    for (const [canonicalMethod, label] of [
      ["two_stage", "Two-stage"],
      ["product_indicator", "Product indicator"],
      ["orthogonalizing", "Orthogonalizing"],
    ] as const) {
      const node = projectedNode({
        semantic: "interaction",
        interaction: {
          termId: `interaction:${canonicalMethod}`,
          predictor: "predictor",
          moderator: "moderator",
          outcome: "outcome",
          method: "two_stage_product_score",
          canonicalMethod,
        },
      });
      const html = renderToStaticMarkup(<NativeModelInspector
        initialTab="parameter"
        nodesOverride={[node]}
        selectedNodeIdOverride={node.id}
        selectedEdgeIdOverride={null}
      />);

      expect(html).toContain(`<dt>Parameter</dt><dd>${label}</dd>`);
    }
  });

  it("renders every projected higher-order approach and measurement type exactly", () => {
    for (const [measurementType, label] of [
      ["reflective_reflective", "Reflective–reflective higher-order construct"],
      ["reflective_formative", "Reflective–formative higher-order construct"],
      ["formative_reflective", "Formative–reflective higher-order construct"],
      ["formative_formative", "Formative–formative higher-order construct"],
    ] as const) {
      const node = projectedNode({
        semantic: "higher_order",
        higherOrder: {
          id: `higher-order:${measurementType}`,
          components: ["component-a", "component-b"],
          method: "two_stage",
          canonicalApproach: "disjoint_two_stage",
          measurementType,
        },
      });
      const html = renderToStaticMarkup(<NativeModelInspector
        nodesOverride={[node]}
        selectedNodeIdOverride={node.id}
        selectedEdgeIdOverride={null}
      />);

      expect(html).toContain(`<dt>Type</dt><dd>${label}</dd>`);
      expect(html).toContain("<dt>Method</dt><dd>Disjoint two-stage</dd>");
      expect(html).toContain("<dt>Indicators</dt><dd>Generated component scores</dd>");
    }

    for (const [canonicalApproach, label] of [
      ["repeated_indicators", "Repeated indicators"],
      ["extended_repeated_indicators", "Extended repeated indicators"],
      ["embedded_two_stage", "Embedded two-stage"],
      ["disjoint_two_stage", "Disjoint two-stage"],
      ["hybrid", "Hybrid"],
    ] as const) {
      const node = projectedNode({
        semantic: "higher_order",
        higherOrder: {
          id: `higher-order:${canonicalApproach}`,
          components: ["component-a", "component-b"],
          method: "two_stage",
          canonicalApproach,
          measurementType: "reflective_reflective",
        },
      });
      const html = renderToStaticMarkup(<NativeModelInspector
        initialTab="parameter"
        nodesOverride={[node]}
        selectedNodeIdOverride={node.id}
        selectedEdgeIdOverride={null}
      />);

      expect(html).toContain(`<dt>Method</dt><dd>${label}</dd>`);
    }
  });

  it("cycles tab focus predictably with Windows keyboard conventions", () => {
    expect(nextNativeModelInspectorTab("model", "ArrowLeft")).toBe("data-binding");
    expect(nextNativeModelInspectorTab("model", "ArrowRight")).toBe("parameter");
    expect(nextNativeModelInspectorTab("appearance", "Home")).toBe("model");
    expect(nextNativeModelInspectorTab("parameter", "End")).toBe("data-binding");
    expect(nextNativeModelInspectorTab("parameter", "PageDown")).toBe("parameter");
  });

  it("routes strict scientific inspector controls through one authority commit seam", () => {
    const source = readFileSync("src/native/NativeModelInspector.tsx", "utf8");
    expect(source).toContain("commitStandardSemModelV4Intent");
    for (const kind of ["rename_construct", "set_construct_representation", "replace_relationship", "assign_indicators", "remove_indicator", "delete_construct", "delete_relationship"]) {
      expect(source).toContain(`kind: \"${kind}\"`);
    }
    expect(source).toContain('role={authorityFeedback.tone === "blocked" || authorityFeedback.tone === "rejected" ? "alert" : "status"}');
  });
});
