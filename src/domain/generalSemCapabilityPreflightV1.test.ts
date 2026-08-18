import { describe, expect, it } from "vitest";
import { defaultGeneralSemConfigV1, type GeneralSemConfigV1 } from "./generalSemConfigV1";
import {
  GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
  GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
  preflightGeneralSemCbsemV1,
  preflightGeneralSemPlsV1,
} from "./generalSemCapabilityPreflightV1";
import {
  convertLegacyBasicModelV4,
  type LegacyBasicModelInterpretationV4,
  type SemModelV4,
} from "./semModelV4";

function model(
  paths: Array<[string, string]> = [["x", "m"], ["m", "y"], ["x", "y"]],
  interpretation: LegacyBasicModelInterpretationV4 = "pls_composite",
): SemModelV4 {
  const constructIds = [...new Set(paths.flat())].sort();
  return convertLegacyBasicModelV4({
    id: "model:preflight",
    name: "General SEM preflight",
    constructs: constructIds.map((id) => ({
      id,
      name: id.toUpperCase(),
      short_name: id.toUpperCase(),
      mode: "reflective",
      indicators: [`${id}1`, `${id}2`],
    })),
    paths: paths.map(([source, target]) => ({ source, target })),
  }, interpretation);
}

function addFeedback(value: SemModelV4): SemModelV4 {
  const feedback = structuredClone(value);
  const targetVariable = feedback.variables.find((variable) => variable.id === "construct:x");
  if (targetVariable?.kind === "common_factor") {
    const varianceParameterId = targetVariable.disturbance_policy.parameter;
    targetVariable.disturbance_policy = {
      kind: "endogenous_disturbance",
      parameter: varianceParameterId,
    };
    const varianceParameter = feedback.parameters.find((parameter) => (
      parameter.id === varianceParameterId && parameter.target.kind === "variance"
    ));
    if (varianceParameter?.target.kind === "variance") {
      varianceParameter.target.endpoint = { kind: "disturbance_of", id: targetVariable.id };
    }
  }
  feedback.relations.push({
    kind: "structural",
    id: "relation:feedback",
    source: "construct:m",
    target: "construct:x",
    parameter: "parameter:feedback",
    intercept_parameter: null,
  });
  feedback.parameters.push({
    kind: "free",
    id: "parameter:feedback",
    label: "M to X",
    target: { kind: "regression", source: "construct:m", target: "construct:x" },
    group_overrides: [],
  });
  return feedback;
}

function codes(decision: ReturnType<typeof preflightGeneralSemPlsV1>): string[] {
  return decision.diagnostics.map((diagnostic) => diagnostic.code);
}

describe("General SEM capability preflight v1", () => {
  it("admits only the recursive composite PLS point-estimation slice to the exact Labs cell", () => {
    const inputModel = model();
    const config = defaultGeneralSemConfigV1();
    const modelBefore = structuredClone(inputModel);
    const configBefore = structuredClone(config);

    const decision = preflightGeneralSemPlsV1(inputModel, config);

    expect(decision).toMatchObject({
      status: "experimental",
      status_label: "Experimental",
      estimator_id: GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
      capability_cells: [{
        registry_schema_version: 2,
        capability_id: "smartpls.mediation",
        cell_id: "qpls3.pls.mediation",
        capability_version: "pls_mediation_v1",
      }],
    });
    expect(codes(decision)).toEqual(["sem.capability.pls.experimental_labs"]);
    expect(Object.isFrozen(decision)).toBe(true);
    expect(inputModel).toEqual(modelBefore);
    expect(config).toEqual(configBefore);
  });

  it("detects feedback by deterministic SCCs regardless of declaration order", () => {
    const feedback = addFeedback(model());
    const reordered = structuredClone(feedback);
    reordered.variables.reverse();
    reordered.relations.reverse();
    reordered.parameters.reverse();

    const expected = preflightGeneralSemPlsV1(feedback, defaultGeneralSemConfigV1());
    const actual = preflightGeneralSemPlsV1(reordered, defaultGeneralSemConfigV1());

    expect(actual).toEqual(expected);
    expect(expected.status).toBe("blocked");
    expect(codes(expected)).toContain("sem.capability.pls.feedback_blocked");
    expect(expected.diagnostics.find((item) => item.code.endsWith("feedback_blocked"))?.corrections)
      .not.toEqual([]);
  });

  it("blocks common factors and authored derived shapes without deleting them", () => {
    const factorModel = model(undefined, "cbsem_common_factor");
    const derivedModel = model();
    derivedModel.variables.push({ kind: "derived", id: "derived:x_squared", label: "X squared" });
    derivedModel.derived_terms.push({
      kind: "polynomial",
      id: "term:x_squared",
      output: "derived:x_squared",
      source: "construct:x",
      degree: 2,
    });
    const derivedBefore = structuredClone(derivedModel);

    expect(codes(preflightGeneralSemPlsV1(factorModel, defaultGeneralSemConfigV1())))
      .toContain("sem.capability.pls.common_factor_not_executable");
    expect(codes(preflightGeneralSemPlsV1(derivedModel, defaultGeneralSemConfigV1())))
      .toContain("sem.capability.pls.derived_shape_not_executable");
    expect(derivedModel).toEqual(derivedBefore);
  });

  it("admits percentile two-sided bootstrap only with both exact capability cells", () => {
    const config = defaultGeneralSemConfigV1();
    config.inference = {
      kind: "case_bootstrap",
      resamples: 500,
      seed: 7,
      confidence_level: 0.95,
      interval: "percentile",
      tail: "two_sided",
    };

    const decision = preflightGeneralSemPlsV1(model(), config);

    expect(decision.status).toBe("experimental");
    expect(decision.capability_cells).toEqual(expect.arrayContaining([
      expect.objectContaining({
        capability_id: "smartpls.mediation",
        cell_id: "qpls3.pls.mediation",
        capability_version: "pls_mediation_v1",
      }),
      expect.objectContaining({
        capability_id: "smartpls.pls_bootstrapping",
        cell_id: "qpls3.inference.bootstrap",
        capability_version: "indexed_resampling_v4",
      }),
    ]));
    expect(decision.capability_cells).toHaveLength(2);
    expect(decision.evidence.map((item) => item.evidence_id)).toEqual(expect.arrayContaining([
      "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1",
      "capability_registry_v2:smartpls.pls_bootstrapping:qpls3.inference.bootstrap:indexed_resampling_v4",
    ]));
    expect(decision.explanation).toContain("matching complete-model");
  });

  it("blocks BCa and one-sided bootstrap with corrective typed diagnostics", () => {
    const cases: Array<[
      GeneralSemConfigV1["inference"],
      string,
    ]> = [
      [{
        kind: "case_bootstrap",
        resamples: 500,
        seed: 7,
        confidence_level: 0.95,
        interval: "bca",
        tail: "two_sided",
      }, "sem.capability.pls.general_bootstrap_bca_not_executable"],
      [{
        kind: "case_bootstrap",
        resamples: 500,
        seed: 7,
        confidence_level: 0.95,
        interval: "percentile",
        tail: "one_sided_lower",
      }, "sem.capability.pls.general_bootstrap_one_sided_not_executable"],
    ];

    for (const [inference, expectedCode] of cases) {
      const config = defaultGeneralSemConfigV1();
      config.inference = inference;
      const decision = preflightGeneralSemPlsV1(model(), config);
      expect(decision.status).toBe("blocked");
      expect(codes(decision)).toContain(expectedCode);
      expect(decision.capability_cells).toHaveLength(2);
      expect(decision.diagnostics
        .filter((diagnostic) => diagnostic.severity === "error")
        .every((diagnostic) => diagnostic.corrections.length > 0)).toBe(true);
    }
  });

  it("blocks probes and every lazy-output request with distinct guidance", () => {
    const config: GeneralSemConfigV1 = defaultGeneralSemConfigV1();
    config.conditional_effect_probes = [{
      probe_id: "probe:m",
      moderator_id: "construct:m",
      values: { kind: "data_derived_mean_plus_minus_one_sd" },
    }];
    config.inference = {
      kind: "case_bootstrap",
      resamples: 500,
      seed: 7,
      confidence_level: 0.95,
      interval: "percentile",
      tail: "two_sided",
    };
    config.output_policy = {
      max_materialized_specific_paths: 100,
      lazy_specific_path_materialization: true,
      when_specific_path_limit_exceeded: "return_lazy",
    };

    const decision = preflightGeneralSemPlsV1(model(), config);

    expect(decision.status).toBe("blocked");
    expect(codes(decision)).toEqual(expect.arrayContaining([
      "sem.capability.pls.conditional_probes_not_executable",
      "sem.capability.pls.lazy_path_materialization_not_executable",
    ]));
    expect(codes(decision)).not.toContain("sem.capability.pls.general_inference_not_executable");
    expect(decision.capability_cells).toHaveLength(2);
    expect(decision.diagnostics.every((diagnostic) => diagnostic.corrections.length > 0)).toBe(true);
  });

  it("fails closed when eager specific paths exceed their explicit resource limit", () => {
    const branchingModel = model([
      ["x", "m1"],
      ["x", "m2"],
      ["m1", "y"],
      ["m2", "y"],
    ]);
    const config = defaultGeneralSemConfigV1();
    config.output_policy.max_materialized_specific_paths = 1;

    const decision = preflightGeneralSemPlsV1(branchingModel, config);

    expect(decision.status).toBe("blocked");
    expect(codes(decision)).toContain("sem.capability.pls.specific_path_limit_exceeded");
    expect(decision.explanation).toContain("remains intact");
  });

  it("rejects requested paths and aggregate effects not present in the compiled topology", () => {
    const config = defaultGeneralSemConfigV1();
    config.requested_effect_estimands = [
      {
        kind: "specific_path",
        estimand_id: "effect:missing_path",
        ordered_relation_ids: ["relation:missing:1", "relation:missing:2"],
      },
      {
        kind: "total_effect",
        estimand_id: "effect:unreachable",
        source_id: "construct:y",
        target_id: "construct:x",
      },
    ];

    const decision = preflightGeneralSemPlsV1(model(), config);

    expect(codes(decision)).toEqual(expect.arrayContaining([
      "sem.capability.pls.requested_path_missing",
      "sem.capability.pls.requested_effect_unreachable",
    ]));
  });

  it("blocks an aggregate estimand that collides with the Rust canonical path identity", () => {
    const config = defaultGeneralSemConfigV1();
    config.requested_effect_estimands = [{
      kind: "total_indirect",
      estimand_id: "sem_specific_path_v1_723ffdb343c774963a152521e697f957280d126da800686f5ced5bb56addb9ab",
      source_id: "construct:x",
      target_id: "construct:y",
    }];

    const decision = preflightGeneralSemPlsV1(model(), config);

    expect(decision.status).toBe("blocked");
    expect(codes(decision)).toContain("sem.capability.pls.effect_identity_collision");
  });

  it("keeps CB-SEM General v3 blocked and gives feedback-specific recovery", () => {
    const recursive = model(undefined, "cbsem_common_factor");
    const runtimeDecision = preflightGeneralSemCbsemV1(recursive, defaultGeneralSemConfigV1());
    expect(runtimeDecision).toMatchObject({
      status: "blocked",
      estimator_id: GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
      capability_cells: [{ cell_id: "qpls3.cbsem.ml" }],
    });
    expect(runtimeDecision.diagnostics.map((item) => item.code))
      .toContain("sem.capability.cbsem.general_runtime_not_connected");

    const feedbackDecision = preflightGeneralSemCbsemV1(
      addFeedback(recursive),
      defaultGeneralSemConfigV1(),
    );
    expect(feedbackDecision.diagnostics.map((item) => item.code))
      .toContain("sem.capability.cbsem.feedback_execution_blocked");
    expect(feedbackDecision.diagnostics.map((item) => item.code))
      .not.toContain("sem.capability.cbsem.general_runtime_not_connected");

    const bootstrapConfig = defaultGeneralSemConfigV1();
    bootstrapConfig.inference = {
      kind: "case_bootstrap",
      resamples: 500,
      seed: 7,
      confidence_level: 0.95,
      interval: "percentile",
      tail: "two_sided",
    };
    const bootstrapDecision = preflightGeneralSemCbsemV1(recursive, bootstrapConfig);
    expect(bootstrapDecision.status).toBe("blocked");
    expect(bootstrapDecision.capability_cells).toEqual([
      expect.objectContaining({ cell_id: "qpls3.cbsem.ml" }),
    ]);
  });
});
