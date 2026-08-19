import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { defaultGeneralSemConfigV1, type GeneralSemConfigV1 } from "./generalSemConfigV1";
import {
  GENERAL_SEM_CBSEM_ESTIMATOR_ID_V1,
  GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CELL_V1,
  GENERAL_SEM_PLS_ESTIMATOR_ID_V1,
  interactionProductColumnIdentityV1,
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

function multipleMediationModel(): SemModelV4 {
  return model([
    ["x", "m1"],
    ["x", "m2"],
    ["x", "y"],
    ["m1", "m2"],
    ["m1", "y"],
    ["m2", "y"],
  ]);
}

function addTwoWayInteraction(
  value: SemModelV4,
  id: string,
  focalPredictor: string,
  moderator: string,
  outcome = "construct:y",
): void {
  const focalRelation = value.relations.find((relation) => (
    relation.kind === "structural"
    && relation.source === focalPredictor
    && relation.target === outcome
  ));
  if (!focalRelation) throw new Error(`Missing focal relation ${focalPredictor} -> ${outcome}`);
  const output = `derived:${id}`;
  const relationId = `relation:${id}:effect`;
  const parameterId = `parameter:${id}:effect`;
  value.variables.push({ kind: "derived", id: output, label: `${focalPredictor} × ${moderator}` });
  value.relations.push({
    kind: "structural",
    id: relationId,
    source: output,
    target: outcome,
    parameter: parameterId,
    intercept_parameter: null,
  });
  value.parameters.push({
    kind: "free",
    id: parameterId,
    label: `${id} effect`,
    target: { kind: "regression", source: output, target: outcome },
    group_overrides: [],
  });
  value.derived_terms.push({
    kind: "interaction_v2",
    id,
    output,
    operands: [focalPredictor, moderator],
    focal_relation: focalRelation.id,
    method: "two_stage",
    hierarchy_policy: "strong",
  });
}

function multipleModerationModel(layout: "same_focal" | "different_focal"): SemModelV4 {
  const value = model([
    ["w", "y"],
    ["x", "y"],
    ["z", "y"],
  ]);
  addTwoWayInteraction(value, "interaction:x:w", "construct:x", "construct:w");
  addTwoWayInteraction(
    value,
    layout === "same_focal" ? "interaction:x:z" : "interaction:z:w",
    layout === "same_focal" ? "construct:x" : "construct:z",
    layout === "same_focal" ? "construct:z" : "construct:w",
  );
  return value;
}

function modelWithSamplingControl(): SemModelV4 {
  const value = model();
  value.variables.push({
    kind: "observed",
    id: "observed:sampling_control",
    label: "Sampling control",
    source_column: "sampling_control",
    scale: "continuous",
    role: "control",
    categories: [],
    value_labels: {},
    missing_markers: [],
    transformation_lineage: [],
  });
  return value;
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
  it("uses UTF-8 byte lengths for deterministic non-ASCII product-column identities", () => {
    const input = {
      interactionId: "interaction:prévision:w",
      outputId: "derived:交互",
      operands: ["construct:prévision", "construct:w"] as const,
      focalRelationId: "relation:prévision:y",
      effectRelationId: "relation:交互:y",
    };
    const digest = createHash("sha256");
    digest.update(Buffer.from("qpls.compiled-pls-plan-v3.two-stage-product\0", "utf8"));
    for (const value of [
      input.interactionId,
      input.outputId,
      ...input.operands,
      input.focalRelationId,
      input.effectRelationId,
    ]) {
      const encoded = Buffer.from(value, "utf8");
      const length = Buffer.alloc(8);
      length.writeBigUInt64BE(BigInt(encoded.byteLength));
      digest.update(length);
      digest.update(encoded);
    }

    expect(Buffer.byteLength(input.outputId, "utf8")).toBeGreaterThan(input.outputId.length);
    expect(interactionProductColumnIdentityV1(input))
      .toBe(`qpls_pls_product_v1_${digest.digest("hex")}`);
  });

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

  it("does not label a direct-only recursive graph as the mediation point cell", () => {
    const decision = preflightGeneralSemPlsV1(
      model([["x", "y"]]),
      defaultGeneralSemConfigV1(),
    );

    expect(decision.status).toBe("blocked");
    expect(codes(decision)).toContain("sem.capability.pls.mediation_requires_indirect_path");
    expect(decision.diagnostics.find((diagnostic) => (
      diagnostic.code === "sem.capability.pls.mediation_requires_indirect_path"
    ))?.corrections).not.toEqual([]);
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

  it("blocks authored missing markers and transformation lineage without rewriting them", () => {
    const missingMarkerModel = model();
    const missingMarkerVariable = missingMarkerModel.variables.find((variable) => (
      variable.kind === "observed" && variable.source_column === "x1"
    ));
    if (missingMarkerVariable?.kind !== "observed") throw new Error("x1 fixture missing");
    missingMarkerVariable.missing_markers = ["-999"];

    const transformedModel = model();
    const transformedVariable = transformedModel.variables.find((variable) => (
      variable.kind === "observed" && variable.source_column === "x1"
    ));
    if (transformedVariable?.kind !== "observed") throw new Error("x1 fixture missing");
    transformedVariable.transformation_lineage = [{
      id: "transform:x1:mean_center",
      input_columns: ["x1_raw"],
      output_column: "x1",
      operation: { kind: "mean_center" },
    }];

    for (const inputModel of [missingMarkerModel, transformedModel]) {
      const before = structuredClone(inputModel);
      const decision = preflightGeneralSemPlsV1(inputModel, defaultGeneralSemConfigV1());

      expect(decision.status).toBe("blocked");
      expect(decision.diagnostics).toContainEqual(expect.objectContaining({
        code: "sem.capability.pls.observed_semantics_not_executable",
        subject: "observed:x1",
      }));
      expect(inputModel).toEqual(before);
    }
  });

  it("blocks non-listwise deletion and each complex-sampling role", () => {
    const nonListwise = model();
    if (nonListwise.data_binding.kind !== "raw") throw new Error("raw fixture required");
    nonListwise.data_binding.missing_data = "mean_replacement";
    expect(codes(preflightGeneralSemPlsV1(nonListwise, defaultGeneralSemConfigV1())))
      .toContain("sem.capability.pls.listwise_deletion_required");

    const complexSamplingModels = [
      (() => {
        const value = modelWithSamplingControl();
        if (value.data_binding.kind !== "raw") throw new Error("raw fixture required");
        value.data_binding.weight = { kind: "case", variable: "observed:sampling_control" };
        return value;
      })(),
      (() => {
        const value = modelWithSamplingControl();
        if (value.data_binding.kind !== "raw") throw new Error("raw fixture required");
        value.data_binding.cluster_variable = "observed:sampling_control";
        return value;
      })(),
      (() => {
        const value = modelWithSamplingControl();
        if (value.data_binding.kind !== "raw") throw new Error("raw fixture required");
        value.data_binding.strata_variable = "observed:sampling_control";
        return value;
      })(),
    ];

    for (const inputModel of complexSamplingModels) {
      const before = structuredClone(inputModel);
      const decision = preflightGeneralSemPlsV1(inputModel, defaultGeneralSemConfigV1());

      expect(decision.status).toBe("blocked");
      expect(codes(decision)).toContain("sem.capability.pls.complex_sampling_not_executable");
      expect(inputModel).toEqual(before);
    }
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

    const decision = preflightGeneralSemPlsV1(multipleMediationModel(), config);

    expect(decision.status).toBe("experimental");
    expect(decision.capability_cells).toEqual(expect.arrayContaining([
      expect.objectContaining({
        capability_id: "smartpls.mediation",
        cell_id: "qpls3.pls.mediation",
        capability_version: "pls_mediation_v1",
      }),
      expect.objectContaining({
        capability_id: "smartpls.mediation",
        cell_id: "qpls3.pls.general_sem_multiple_mediation_bootstrap",
        capability_version: "general_sem_pls_full_model_case_bootstrap_v1",
      }),
    ]));
    expect(decision.capability_cells).toHaveLength(2);
    expect(decision.evidence.map((item) => item.evidence_id)).toEqual(expect.arrayContaining([
      "compiler:recipe_v4_to_compiled_pls_plan_v3_bootstrap_v1",
      "capability_registry_v2:smartpls.mediation:qpls3.pls.general_sem_multiple_mediation_bootstrap:general_sem_pls_full_model_case_bootstrap_v1",
      "capability_dependency:smartpls.pls_bootstrapping:qpls3.inference.bootstrap:indexed_resampling_v4",
    ]));
    expect(decision.explanation).toContain("matching complete-model");
  });

  it.each(["same_focal", "different_focal"] as const)(
    "admits simultaneous two-way moderation on %s paths only to the exact point cell",
    (layout) => {
      const inputModel = multipleModerationModel(layout);
      const before = structuredClone(inputModel);
      const decision = preflightGeneralSemPlsV1(inputModel, defaultGeneralSemConfigV1());

      expect(decision.status).toBe("experimental");
      expect(decision.capability_cells).toStrictEqual([
        GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CELL_V1,
      ]);
      expect(codes(decision)).toEqual(["sem.capability.pls.experimental_labs"]);
      expect(decision.summary).toContain("Experimental Labs");
      expect(decision.explanation).toContain("joint stage-two solve");
      expect(inputModel).toStrictEqual(before);
    },
  );

  it("keeps simultaneous interaction bootstrap blocked with the exact corrective cell decision", () => {
    const config = defaultGeneralSemConfigV1();
    config.inference = {
      kind: "case_bootstrap",
      resamples: 500,
      seed: 7,
      confidence_level: 0.95,
      interval: "percentile",
      tail: "two_sided",
    };

    const decision = preflightGeneralSemPlsV1(multipleModerationModel("same_focal"), config);

    expect(decision.status).toBe("blocked");
    expect(decision.capability_cells).toEqual(expect.arrayContaining([
      GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_CELL_V1,
      expect.objectContaining({
        cell_id: "qpls3.pls.general_sem_multiple_mediation_bootstrap",
      }),
    ]));
    expect(codes(decision)).toContain(
      "sem.capability.pls.multiple_moderation_bootstrap_not_executable",
    );
    expect(decision.diagnostics.find((diagnostic) => (
      diagnostic.code === "sem.capability.pls.multiple_moderation_bootstrap_not_executable"
    ))?.corrections.join(" ")).toContain("inference to none");
  });

  it("blocks a single indirect path from the exact multiple-mediation bootstrap cell", () => {
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

    expect(decision.status).toBe("blocked");
    expect(codes(decision)).toContain(
      "sem.capability.pls.multiple_mediation_requires_two_indirect_paths",
    );
    expect(decision.diagnostics.find((diagnostic) => (
      diagnostic.code === "sem.capability.pls.multiple_mediation_requires_two_indirect_paths"
    ))?.corrections).not.toEqual([]);
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
      const decision = preflightGeneralSemPlsV1(multipleMediationModel(), config);
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
