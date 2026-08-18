import type { ReactElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { defaultGeneralSemConfigV1 } from "../domain/generalSemConfigV1";
import {
  GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
  GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
  preflightGeneralSemCbsemV1,
  preflightGeneralSemPlsV1,
} from "../domain/generalSemCapabilityPreflightV1";
import {
  convertLegacyBasicModelV4,
  type SemModelV4,
} from "../domain/semModelV4";
import {
  GeneralSemEstimatorCompatibilityPanel,
  GeneralSemEstimatorSelectionButton,
} from "./GeneralSemEstimatorCompatibilityPanel";

function recursivePlsModel(): SemModelV4 {
  return convertLegacyBasicModelV4({
    id: "model:compatibility-panel",
    name: "Mediation model",
    constructs: ["x", "m", "y"].map((id) => ({
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
    ],
  }, "pls_composite");
}

describe("GeneralSemEstimatorCompatibilityPanel", () => {
  it("renders both exact estimator decisions with visible live status and full recovery text", () => {
    const html = renderToStaticMarkup(<GeneralSemEstimatorCompatibilityPanel
      model={recursivePlsModel()}
      config={defaultGeneralSemConfigV1()}
      onSelectEstimator={() => undefined}
    />);

    expect(html).toContain('data-general-sem-estimator-compatibility="v1"');
    expect(html).toContain("Estimator compatibility preview");
    expect(html).toContain("The native compiler confirms the final capability decision before calculation.");
    expect(html.match(/data-general-sem-estimator-card=/g)).toHaveLength(2);
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-live="polite"');
    expect(html).toContain('aria-atomic="true"');
    expect(html).toContain("PLS-SEM General v3: Experimental");
    expect(html).toContain("CB-SEM General v3: Blocked");
    expect(html).toContain("PLS-SEM can calculate this request in Experimental Labs.");
    expect(html).toContain("The complete recursive model is re-estimated by the proven PLS score executor");
    expect(html).toContain("Experimental Labs.</strong> This request is runnable only through the exact experimental capability");
    expect(html).toContain("Diagnostics and next actions");
    expect(html).toContain("How to proceed");
    expect(html).toContain("Cannot select:");
    expect(html).toMatch(/<button(?=[^>]*data-general-sem-estimator-select="qpls\.pls_sem\.v3")(?![^>]*disabled="")[^>]*>/);
    expect(html).toMatch(/<button(?=[^>]*data-general-sem-estimator-select="qpls\.cbsem\.v3")(?=[^>]*disabled="")[^>]*>/);
  });

  it("exposes selected state as text and aria-pressed without selecting a blocked estimator", () => {
    const selectedHtml = renderToStaticMarkup(<GeneralSemEstimatorCompatibilityPanel
      model={recursivePlsModel()}
      config={defaultGeneralSemConfigV1()}
      selectedEstimatorId={GENERAL_SEM_PLS_ESTIMATOR_ID_V1}
      onSelectEstimator={() => undefined}
    />);
    expect(selectedHtml).toContain("Selected: PLS-SEM General v3.");
    expect(selectedHtml).toMatch(/<button(?=[^>]*data-general-sem-estimator-select="qpls\.pls_sem\.v3")(?=[^>]*aria-pressed="true")[^>]*>Selected PLS-SEM General v3<\/button>/);

    const blockedSelectedHtml = renderToStaticMarkup(<GeneralSemEstimatorCompatibilityPanel
      model={recursivePlsModel()}
      config={defaultGeneralSemConfigV1()}
      selectedEstimatorId={GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1}
      onSelectEstimator={() => undefined}
    />);
    expect(blockedSelectedHtml).toContain("No runnable estimator selected.");
    expect(blockedSelectedHtml).not.toContain("Selected CB-SEM General v3");
  });

  it("disables both controls and renders actionable inference guidance when PLS is blocked", () => {
    const config = defaultGeneralSemConfigV1();
    config.inference = {
      kind: "case_bootstrap",
      resamples: 500,
      seed: 7,
      confidence_level: 0.95,
      interval: "percentile",
      tail: "two_sided",
    };
    const html = renderToStaticMarkup(<GeneralSemEstimatorCompatibilityPanel
      model={recursivePlsModel()}
      config={config}
      onSelectEstimator={() => undefined}
    />);

    expect(html.match(/<button[^>]*disabled=""[^>]*data-general-sem-estimator-select=/g)).toHaveLength(2);
    expect(html).toContain("General SEM case-bootstrap inference is requested but is not connected");
    expect(html).toContain("Set General SEM inference to none for the current point-estimation slice");
    expect(html).toContain('aria-disabled="true"');
  });

  it("wires selection only for Supported or Experimental decisions", () => {
    const model = recursivePlsModel();
    const config = defaultGeneralSemConfigV1();
    const onSelectEstimator = vi.fn();
    const experimental = preflightGeneralSemPlsV1(model, config);
    const blocked = preflightGeneralSemCbsemV1(model, config);

    const runnableButton = GeneralSemEstimatorSelectionButton({
      estimatorId: GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
      estimatorLabel: "PLS-SEM General v3",
      decision: experimental,
      selected: false,
      descriptionId: "pls-description",
      blockedReason: "",
      onSelectEstimator,
    }) as ReactElement<{ disabled: boolean; onClick?: () => void }>;
    expect(runnableButton.props.disabled).toBe(false);
    expect(runnableButton.props.onClick).toBeTypeOf("function");
    runnableButton.props.onClick?.();
    expect(onSelectEstimator).toHaveBeenCalledOnce();
    expect(onSelectEstimator).toHaveBeenCalledWith(GENERAL_SEM_PLS_ESTIMATOR_ID_V1);

    const blockedButton = GeneralSemEstimatorSelectionButton({
      estimatorId: GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
      estimatorLabel: "CB-SEM General v3",
      decision: blocked,
      selected: false,
      descriptionId: "cbsem-description",
      blockedReason: blocked.summary,
      onSelectEstimator,
    }) as ReactElement<{ disabled: boolean; onClick?: () => void }>;
    expect(blockedButton.props.disabled).toBe(true);
    expect(blockedButton.props.onClick).toBeUndefined();
    expect(onSelectEstimator).toHaveBeenCalledOnce();
  });
});
