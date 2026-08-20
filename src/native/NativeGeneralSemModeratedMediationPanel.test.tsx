import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { defaultGeneralSemConfigV1 } from "../domain/generalSemConfigV1";
import { canonicalizeSemModelV4, convertLegacyBasicModelV4 } from "../domain/semModelV4";
import { NativeGeneralSemModeratedMediationPanel } from "./NativeGeneralSemModeratedMediationPanel";

function fixture() {
  const model = convertLegacyBasicModelV4({
    id: "model:moderated-mediation-ui",
    name: "Moderated mediation",
    constructs: ["m", "w", "x", "y"].map((id) => ({
      id,
      name: id.toUpperCase(),
      short_name: id.toUpperCase(),
      mode: "reflective" as const,
      indicators: [`${id}1`, `${id}2`],
    })),
    paths: [
      { source: "x", target: "m" },
      { source: "m", target: "y" },
      { source: "x", target: "y" },
      { source: "w", target: "m" },
    ],
  }, "pls_composite");
  const focal = model.relations.find((relation) => relation.kind === "structural"
    && relation.source === "construct:x" && relation.target === "construct:m");
  if (!focal) throw new Error("focal fixture missing");
  model.variables.push({ kind: "derived", id: "derived:x-w", label: "X by W" });
  model.relations.push({
    kind: "structural",
    id: "relation:x-w:m",
    source: "derived:x-w",
    target: "construct:m",
    parameter: "parameter:x-w:m",
    intercept_parameter: null,
  });
  model.parameters.push({
    kind: "free",
    id: "parameter:x-w:m",
    label: "X by W to M",
    target: { kind: "regression", source: "derived:x-w", target: "construct:m" },
    group_overrides: [],
  });
  model.derived_terms.push({
    kind: "interaction_v2",
    id: "interaction:x-w:m",
    output: "derived:x-w",
    operands: ["construct:x", "construct:w"],
    focal_relation: focal.id,
    method: "two_stage",
    hierarchy_policy: "strong",
    product_indicator: null,
  });
  const config = defaultGeneralSemConfigV1();
  config.inference = {
    kind: "case_bootstrap",
    resamples: 500,
    seed: 42,
    confidence_level: 0.95,
    interval: "percentile",
    tail: "two_sided",
  };
  return { model: canonicalizeSemModelV4(model), config };
}

describe("NativeGeneralSemModeratedMediationPanel", () => {
  it("renders the selected stage, locked probes, and exact five targets for the Labs route", () => {
    const { model, config } = fixture();
    const html = renderToStaticMarkup(<NativeGeneralSemModeratedMediationPanel
      connected
      model={model}
      config={config}
      onSaveAsRevision={vi.fn()}
    />);

    expect(html).toContain("Two-way moderated mediation");
    expect(html).toContain("First stage (X × W → M)");
    expect(html).toContain("Scientific gamma inference");
    expect(html.match(/Conditional indirect effect at W =/gu)).toHaveLength(3);
    expect(html).toContain("Index of moderated mediation");
    expect(html).toContain("Save path as new model + Recipe revision");
    expect(html).toContain("The current archive remains unchanged");
  });

  it("stays hidden when the product connection is false", () => {
    const { model, config } = fixture();
    const html = renderToStaticMarkup(<NativeGeneralSemModeratedMediationPanel
      connected={false}
      model={model}
      config={config}
    />);

    expect(html).toContain("hidden");
    expect(html).toContain("moderated-mediation-unavailable");
    expect(html).not.toContain("Save path as new model + Recipe revision");
  });

  it("renders a blocked recovery state without enabling another revision write", () => {
    const { model, config } = fixture();
    const html = renderToStaticMarkup(<NativeGeneralSemModeratedMediationPanel
      connected
      model={model}
      config={config}
      revisionBlocked
      revisionBlockedReason="Reopen the exact clean source before retrying."
      revisionStatusMessage="The saved destination was not activated."
      revisionFailure={{
        code: "schema6_general_sem_revision_v2.activation_failed_source_restored",
        message: "QuickPLS restored the source authority.",
        correctiveAction: "Open the saved revision explicitly when ready.",
      }}
      onSaveAsRevision={vi.fn()}
    />);

    expect(html).toContain("QuickPLS restored the source authority.");
    expect(html).toContain("The saved destination was not activated.");
    expect(html).toContain("Reopen the exact clean source before retrying.");
    expect(html).toContain("disabled");
  });
});
