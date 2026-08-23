import { renderToStaticMarkup } from "react-dom/server";
import type { Node } from "@xyflow/react";
import { readFileSync } from "node:fs";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { useWorkspace } from "../store";
import { convertLegacyBasicModelV4 } from "../domain/semModelV4";
import {
  NativeModelInspector,
  nativeHigherOrderPositionLabel,
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
    expect(html).not.toContain("Experimental Labs");
    expect(html).not.toContain("Scientific authoring remains in Experimental Labs.");
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
    expect(html).toContain("Scientific representation");
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

  it("shows an expert-only, accessible editor for a selected measurement connector without path actions", () => {
    const construct = useWorkspace.getState().nodes.find((node) => node.data.indicators.length > 0)!;
    const indicator = construct.data.indicators[0]!;
    const selection = { constructId: construct.id, indicator };

    const expert = renderToStaticMarkup(<NativeModelInspector
      initialMode="expert"
      initialTab="appearance"
      selectedNodeIdOverride={null}
      selectedEdgeIdOverride={null}
      selectedMeasurementConnectorOverride={selection}
    />);
    const basic = renderToStaticMarkup(<NativeModelInspector
      initialMode="basic"
      initialTab="appearance"
      selectedNodeIdOverride={null}
      selectedEdgeIdOverride={null}
      selectedMeasurementConnectorOverride={selection}
    />);

    expect(expert).toContain('id="nd-model-inspector-heading">Measurement connector</strong>');
    expect(expert).toContain(`aria-label="Connector route for ${indicator} in ${construct.data.label}"`);
    for (const option of ["Straight (default)", "Curved", "Orthogonal", "Polyline (editable bends)"]) {
      expect(expert).toContain(`>${option}</option>`);
    }
    expect(expert).toContain("Reset connector route");
    expect(expert).toContain("Connector routing is presentation-only; it does not change the scientific measurement relationship or arrow direction.");
    expect(expert).not.toContain(">Reverse</button>");
    expect(expert).not.toContain("Delete relationship");
    expect(basic).toContain("Switch to Expert to change this measurement connector&#x27;s presentation route.");
    expect(basic).not.toContain("Reset connector route");
  });

  it("keeps construct-wide measurement routing compact and Expert-only", () => {
    const construct = useWorkspace.getState().nodes.find((node) => node.data.indicators.length > 0)!;
    const basic = renderToStaticMarkup(<NativeModelInspector
      initialMode="basic"
      initialTab="appearance"
      selectedNodeIdOverride={construct.id}
      selectedEdgeIdOverride={null}
      selectedMeasurementConnectorOverride={null}
    />);
    const expert = renderToStaticMarkup(<NativeModelInspector
      initialMode="expert"
      initialTab="appearance"
      selectedNodeIdOverride={construct.id}
      selectedEdgeIdOverride={null}
      selectedMeasurementConnectorOverride={null}
    />);

    expect(basic).not.toContain("All measurement connectors");
    expect(expert).toContain("All measurement connectors");
    expect(expert).toContain(`aria-label="Routing for all measurement connectors of ${construct.data.label}"`);
    expect(expert).toContain("Reset all connector routes");
    expect(expert).toContain("Select an indicator or connector to edit one route.");
  });

  it("disables measurement connector routing in result and publication diagrams", () => {
    const construct = useWorkspace.getState().nodes.find((node) => node.data.indicators.length > 0)!;
    const indicator = construct.data.indicators[0]!;

    for (const diagramModeOverride of ["smartpls_result", "publication"] as const) {
      const html = renderToStaticMarkup(<NativeModelInspector
        initialMode="expert"
        initialTab="appearance"
        diagramModeOverride={diagramModeOverride}
        selectedNodeIdOverride={null}
        selectedEdgeIdOverride={null}
        selectedMeasurementConnectorOverride={{ constructId: construct.id, indicator }}
      />);

      expect(html).toContain(`disabled="" aria-label="Connector route for ${indicator} in ${construct.data.label}"`);
      expect(html).toContain('type="button" class="nd-secondary-command" disabled="">Reset connector route</button>');
    }
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

  it("reads back ordered interaction_v2 operands, hierarchy, and canonical method", () => {
    const focal = projectedNode({ label: "Focal predictor", shortName: "X" });
    focal.id = "construct:x";
    const firstModerator = projectedNode({ label: "First moderator", shortName: "Z" });
    firstModerator.id = "construct:z";
    const secondModerator = projectedNode({ label: "Second moderator", shortName: "W" });
    secondModerator.id = "construct:w";
    const outcome = projectedNode({ label: "Outcome", shortName: "Y" });
    outcome.id = "construct:y";
    const interaction = projectedNode({
      semantic: "interaction",
      interaction: {
        kind: "interaction_v2",
        termId: "interaction:x-z-w",
        operands: ["construct:x", "construct:z", "construct:w"],
        outcome: "construct:y",
        focalRelationId: "path:x-y",
        canonicalMethod: "orthogonalizing",
        hierarchyPolicy: "strong",
        productIndicator: null,
      },
    });
    const nodes = [focal, firstModerator, secondModerator, outcome, interaction];
    const modelHtml = renderToStaticMarkup(<NativeModelInspector
      nodesOverride={nodes}
      selectedNodeIdOverride={interaction.id}
      selectedEdgeIdOverride={null}
    />);
    const parameterHtml = renderToStaticMarkup(<NativeModelInspector
      initialTab="parameter"
      nodesOverride={nodes}
      selectedNodeIdOverride={interaction.id}
      selectedEdgeIdOverride={null}
    />);

    expect(modelHtml).toContain("<dt>Focal predictor</dt><dd>Focal predictor</dd>");
    expect(modelHtml).toContain("<dt>Moderators (authored order)</dt><dd>First moderator × Second moderator</dd>");
    expect(modelHtml).toContain("<dt>Hierarchy policy</dt><dd>strong</dd>");
    expect(parameterHtml).toContain("<dt>Parameter</dt><dd>Orthogonalizing</dd>");
    expect(parameterHtml).toContain("ordered operand scores");
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
        edgesOverride={[{ id: "component-to-hoc", source: "component-a", target: node.id }]}
        selectedNodeIdOverride={node.id}
        selectedEdgeIdOverride={null}
        onEditHigherOrder={() => undefined}
      />);

      expect(html).toContain(`<dt>Type</dt><dd>${label}</dd>`);
      expect(html).toContain("<dt>Approach</dt><dd>Disjoint two-stage</dd>");
      expect(html).toContain("<dt>Position</dt><dd>Endogenous</dd>");
      expect(html).toContain("<dt>Inputs</dt><dd>Generated component scores</dd>");
      expect(html).toContain(`aria-label="Edit higher-order construct ${node.data.label}"`);
      expect(html).toContain(">Edit…</button>");
    }

    for (const [canonicalApproach, label, inputs] of [
      ["repeated_indicators", "Repeated indicators", "Repeated component indicators"],
      ["extended_repeated_indicators", "Extended repeated indicators", "Extended repeated component indicators"],
      ["embedded_two_stage", "Embedded two-stage", "Generated component scores"],
      ["disjoint_two_stage", "Disjoint two-stage", "Generated component scores"],
      ["hybrid", "Hybrid", "Component indicators and generated scores"],
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

      expect(html).toContain(`<dt>Approach</dt><dd>${label}</dd>`);
      expect(html).toContain(`<dt>Inputs</dt><dd>${inputs}</dd>`);
    }
  });

  it("derives HOC structural position without treating covariance or visual membership as regressions", () => {
    const decorative = { visualOnly: true };
    expect(nativeHigherOrderPositionLabel("hoc", [
      { id: "membership", source: "component", target: "hoc", data: decorative },
      { id: "covariance", source: "peer", target: "hoc", data: { role: "covariance" } },
    ])).toBe("Unconnected");
    expect(nativeHigherOrderPositionLabel("hoc", [
      { id: "hoc-outcome", source: "hoc", target: "outcome" },
    ])).toBe("Exogenous");
    expect(nativeHigherOrderPositionLabel("hoc", [
      { id: "predictor-hoc", source: "predictor", target: "hoc", data: { role: "control" } },
    ])).toBe("Endogenous");
  });

  it("cycles tab focus predictably with Windows keyboard conventions", () => {
    expect(nextNativeModelInspectorTab("model", "ArrowLeft")).toBe("data-binding");
    expect(nextNativeModelInspectorTab("model", "ArrowRight")).toBe("parameter");
    expect(nextNativeModelInspectorTab("appearance", "Home")).toBe("model");
    expect(nextNativeModelInspectorTab("parameter", "End")).toBe("data-binding");
    expect(nextNativeModelInspectorTab("parameter", "PageDown")).toBe("parameter");
  });

  it("routes common inspector edits through the shared gateway and preserves strict-only commits", () => {
    const source = readFileSync("src/native/NativeModelInspector.tsx", "utf8");
    expect(source).toContain("executeModelEditCommand");
    expect(source).toContain("const executeModelEdit = async");
    for (const kind of ["rename_construct", "invert_measurement_model", "assign_indicators", "unassign_indicator", "reverse_path", "remove_path"]) {
      expect(source).toContain(`kind: \"${kind}\"`);
    }
    expect(source).toContain("commitStandardSemModelV4Intent");
    for (const kind of ["set_construct_representation", "replace_relationship", "delete_construct"]) {
      expect(source).toContain(`kind: \"${kind}\"`);
    }
    expect(source).toContain('role={authorityFeedback.tone === "blocked" || authorityFeedback.tone === "rejected" ? "alert" : "status"}');
  });
});
