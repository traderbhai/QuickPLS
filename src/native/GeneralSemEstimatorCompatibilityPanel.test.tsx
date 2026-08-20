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
    name: "Parallel mediation model",
    constructs: ["x", "m1", "m2", "y"].map((id) => ({
      id,
      name: id.toUpperCase(),
      short_name: id.toUpperCase(),
      mode: "reflective" as const,
      indicators: [`${id}1`, `${id}2`],
    })),
    paths: [
      { source: "x", target: "m1" },
      { source: "m1", target: "y" },
      { source: "x", target: "m2" },
      { source: "m2", target: "y" },
      { source: "x", target: "y" },
    ],
  }, "pls_composite");
}

const authority = {
  source: "resident_schema6_sem_model_v4_parameter_table" as const,
  modelId: "model:compatibility-panel",
  modelScientificSha256: "a".repeat(64),
  parameterTableSha256: "b".repeat(64),
  parameterCount: 24,
  freeParameterCount: 20,
  fixedParameterCount: 4,
  derivedParameterCount: 0,
  equalityLabeledParameterCount: 2,
  boundedParameterCount: 3,
  explicitConstraintCount: 0,
};

function decisions(config = defaultGeneralSemConfigV1()) {
  const model = recursivePlsModel();
  return {
    pls: preflightGeneralSemPlsV1(model, config),
    cbsem: preflightGeneralSemCbsemV1(model, config),
  };
}

describe("GeneralSemEstimatorCompatibilityPanel", () => {
  it("renders both exact estimator decisions with visible live status and full recovery text", () => {
    const html = renderToStaticMarkup(<GeneralSemEstimatorCompatibilityPanel
      decisions={decisions()}
      authority={authority}
      onSelectEstimator={() => undefined}
    />);

    expect(html).toContain('data-general-sem-estimator-compatibility="v1"');
    expect(html).toContain("Estimator compatibility");
    expect(html).toContain("Native preflight from the active resident schema-6 SemModelV4 parameter table.");
    expect(html).toContain("24 parameters (20 free, 4 fixed, 0 derived)");
    expect(html).toContain("Compatibility inspection only: a blocked or unpublished candidate has no calculation action.");
    expect(html).not.toContain("likely to calculate");
    expect(html).not.toContain("before calculation");
    expect(html.match(/data-general-sem-estimator-card=/g)).toHaveLength(2);
    expect(html).toContain('role="status"');
    expect(html).toContain('aria-live="polite"');
    expect(html).toContain('aria-atomic="true"');
    expect(html).toContain("PLS-SEM General v3: Experimental");
    expect(html).toContain("CB-SEM General v3: Blocked");
    expect(html).toContain("PLS-SEM can compile this exact Registry-governed request.");
    expect(html).toContain("The compiler binds the proven PLS scoring plan to stable relation-path identities.");
    expect(html).toContain("Experimental Labs.</strong> This request passes the exact compiler-qualification cells listed above.");
    expect(html).toContain("Selecting it records an estimator preference only; it does not start native execution.");
    expect(html).toContain("Exact capability cells: qpls3.pls.mediation (pls_mediation_v1)");
    expect(html).toContain("Diagnostics and next actions");
    expect(html).toContain("How to proceed");
    expect(html).toContain("Cannot select:");
    expect(html).toMatch(/<button(?=[^>]*data-general-sem-estimator-select="qpls\.pls_sem\.v3")(?![^>]*disabled="")[^>]*>/);
    expect(html).toMatch(/<button(?=[^>]*data-general-sem-estimator-select="qpls\.cbsem\.v3")(?=[^>]*disabled="")[^>]*>/);
  });

  it("exposes selected state as text and aria-pressed without selecting a blocked estimator", () => {
    const selectedHtml = renderToStaticMarkup(<GeneralSemEstimatorCompatibilityPanel
      decisions={decisions()}
      authority={authority}
      selectedEstimatorId={GENERAL_SEM_PLS_ESTIMATOR_ID_V1}
      onSelectEstimator={() => undefined}
    />);
    expect(selectedHtml).toContain("Selected: PLS-SEM General v3.");
    expect(selectedHtml).toMatch(/<button(?=[^>]*data-general-sem-estimator-select="qpls\.pls_sem\.v3")(?=[^>]*aria-pressed="true")[^>]*>Selected PLS-SEM General v3<\/button>/);

    const blockedSelectedHtml = renderToStaticMarkup(<GeneralSemEstimatorCompatibilityPanel
      decisions={decisions()}
      authority={authority}
      selectedEstimatorId={GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1}
      onSelectEstimator={() => undefined}
    />);
    expect(blockedSelectedHtml).toContain("No compile-qualified estimator selected.");
    expect(blockedSelectedHtml).not.toContain("Selected CB-SEM General v3");
  });

  it("qualifies supported percentile two-sided bootstrap as Experimental and renders every exact capability cell", () => {
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
      decisions={decisions(config)}
      authority={authority}
      onSelectEstimator={() => undefined}
    />);

    expect(html).toContain("PLS-SEM General v3: Experimental");
    expect(html).toContain("General recursive PLS percentile case-bootstrap inference passes the bounded exact-cell compiler preflight.");
    expect(html).toContain("qpls3.pls.mediation (pls_mediation_v1)");
    expect(html).toContain("qpls3.pls.general_sem_multiple_mediation_bootstrap (general_sem_pls_full_model_case_bootstrap_v1)");
    expect(html).toContain("Runtime inference must carry a matching complete-model re-estimation receipt before publication.");
    expect(html).toContain("Compatibility inspection only: a blocked or unpublished candidate has no calculation action.");
    expect(html.match(/<button[^>]*disabled=""[^>]*data-general-sem-estimator-select=/g)).toHaveLength(1);
    expect(html).toMatch(/<button(?=[^>]*data-general-sem-estimator-select="qpls\.pls_sem\.v3")(?![^>]*disabled="")[^>]*>Select PLS-SEM General v3<\/button>/);
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
