import { describe, expect, it } from "vitest";
import type { CanonicalResultDocumentV2 } from "./canonicalResultDocumentV2";
import { defaultGeneralSemConfigV1 } from "./generalSemConfigV1";
import {
  detectUnifiedSemFeatureInventoryV1,
  resolveUnifiedSemCalculationV1,
  unifiedSemModeratedMediationCandidatesV1,
  type UnifiedSemCalculationContextV1,
} from "./unifiedSemCalculationV1";
import {
  convertLegacyBasicModelV4,
  type LegacyBasicModelInterpretationV4,
  type SemModelV4,
} from "./semModelV4";

const bootstrap = { resamples: 500, seed: 17, confidenceLevel: 0.95 } as const;

function model(
  paths: Array<[string, string]>,
  interpretation: LegacyBasicModelInterpretationV4 = "pls_composite",
): SemModelV4 {
  const constructs = [...new Set(paths.flat())].sort();
  return convertLegacyBasicModelV4({
    id: "model:unified-calculate",
    name: "Unified Calculate",
    constructs: constructs.map((id) => ({
      id,
      name: id.toUpperCase(),
      short_name: id.toUpperCase(),
      mode: "reflective",
      indicators: [`${id}1`, `${id}2`, `${id}3`],
    })),
    paths: paths.map(([source, target]) => ({ source, target })),
  }, interpretation);
}

function context(value: SemModelV4): UnifiedSemCalculationContextV1 {
  return {
    authorityKey: "authority:v1",
    model: value,
    config: defaultGeneralSemConfigV1(),
  };
}

function addTwoWayInteraction(
  value: SemModelV4,
  focalSource: string,
  moderator: string,
  target: string,
): void {
  const focal = value.relations.find((relation) => relation.kind === "structural"
    && relation.source === focalSource
    && relation.target === target);
  if (!focal || focal.kind !== "structural") throw new Error("Missing focal path");
  const output = "derived:interaction";
  value.variables.push({ kind: "derived", id: output, label: "Interaction" });
  value.derived_terms.push({
    kind: "interaction_v2",
    id: "term:interaction",
    output,
    operands: [focalSource, moderator],
    focal_relation: focal.id,
    method: "two_stage",
    hierarchy_policy: "strong",
  });
  value.relations.push({
    kind: "structural",
    id: "relation:interaction-effect",
    source: output,
    target,
    parameter: "parameter:interaction-effect",
    role: "structural",
    intercept_parameter: null,
  });
  value.parameters.push({
    kind: "free",
    id: "parameter:interaction-effect",
    label: "Interaction effect",
    target: { kind: "regression", source: output, target },
    group_overrides: [],
  });
}

describe("unified SEM Calculate routing", () => {
  it("leaves ordinary direct-only PLS on the established workflow", () => {
    const ordinary = context(model([["x", "y"]]));

    const point = resolveUnifiedSemCalculationV1({ method: "pls_algorithm", context: ordinary, bootstrap });
    const inference = resolveUnifiedSemCalculationV1({ method: "pls_bootstrap", context: ordinary, bootstrap });

    expect(point).toMatchObject({ route: "legacy", fallbackReason: "ordinary_model", canStart: true });
    expect(inference).toMatchObject({ route: "legacy", fallbackReason: "ordinary_model", canStart: true });
  });

  it("turns transient invalid inference inputs into a blocking plan instead of throwing", () => {
    const strict = context(model([
      ["x", "m1"], ["m1", "y"],
      ["x", "m2"], ["m2", "y"],
    ]));

    const plan = resolveUnifiedSemCalculationV1({
      method: "pls_bootstrap",
      context: strict,
      bootstrap: { resamples: 500, seed: 17, confidenceLevel: 0 },
    });

    expect(plan).toMatchObject({ route: "general_sem_pls", canStart: false });
    expect(plan.blockers).toEqual([
      "Review the bootstrap samples, confidence level, and seed. The current values do not satisfy this method's bounded inference contract.",
    ]);
  });

  it("routes mediation point and multiple-mediation bootstrap through their unchanged cells", () => {
    const single = context(model([["x", "m"], ["m", "y"], ["x", "y"]]));
    const multiple = context(model([
      ["x", "m1"], ["m1", "y"],
      ["x", "m2"], ["m2", "y"],
      ["x", "y"],
    ]));

    const point = resolveUnifiedSemCalculationV1({ method: "pls_algorithm", context: single, bootstrap });
    const inference = resolveUnifiedSemCalculationV1({ method: "pls_bootstrap", context: multiple, bootstrap });

    expect(point.route).toBe("general_sem_pls");
    expect(point.capabilityCells.map((cell) => cell.cell_id)).toContain("qpls3.pls.mediation");
    expect(point.featureSummaries).toContain("1 indirect path");
    expect(inference.route).toBe("general_sem_pls");
    expect(inference.capabilityCells.map((cell) => cell.cell_id)).toContain(
      "qpls3.pls.general_sem_multiple_mediation_bootstrap",
    );
    expect(inference.requestedConfig?.inference).toMatchObject({
      kind: "case_bootstrap",
      resamples: 500,
      seed: 17,
    });
  });

  it("blocks unsupported single-path mediation bootstrap instead of silently running ordinary bootstrap", () => {
    const single = context(model([["x", "m"], ["m", "y"], ["x", "y"]]));

    const inference = resolveUnifiedSemCalculationV1({
      method: "pls_bootstrap",
      context: single,
      bootstrap,
    });

    expect(inference).toMatchObject({ route: "general_sem_pls", canStart: false });
    expect(inference.blockers.join(" ")).toContain("requires at least two compiled specific indirect paths");
  });

  it("offers fixed five-target moderated mediation only from PLS bootstrapping", () => {
    const moderated = model([
      ["x", "m"],
      ["m", "y"],
      ["x", "y"],
      ["w", "m"],
    ]);
    addTwoWayInteraction(moderated, "construct:x", "construct:w", "construct:m");
    const strict = context(moderated);

    const point = resolveUnifiedSemCalculationV1({ method: "pls_algorithm", context: strict, bootstrap });
    const inference = resolveUnifiedSemCalculationV1({ method: "pls_bootstrap", context: strict, bootstrap });

    expect(point.moderatedMediation).toBeNull();
    expect(point.capabilityCells.map((cell) => cell.cell_id)).toContain(
      "qpls3.pls.general_sem_multiple_two_way_moderation_point",
    );
    expect(inference.moderatedMediation).toMatchObject({
      candidateCount: 1,
      autoSelected: false,
      configurationRequired: true,
      fixedTargetSummary: "1 scientific gamma, 3 conditional indirect effects at W = -1, 0, +1, and 1 index of moderated mediation",
    });
    expect(inference.moderatedMediation?.selectedPath).toBeNull();
    expect(inference.capabilityCells.map((cell) => cell.cell_id)).not.toContain(
      "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
    );

    const selectedPath = unifiedSemModeratedMediationCandidatesV1(strict, bootstrap)[0];
    expect(selectedPath).toBeDefined();
    const selected = resolveUnifiedSemCalculationV1({
      method: "pls_bootstrap",
      context: strict,
      bootstrap,
      moderatedMediationPathId: selectedPath!.pathId,
    });
    expect(selected.moderatedMediation?.selectedPath?.moderatedStage).toBe("first_stage");
    expect(selected.capabilityCells.map((cell) => cell.cell_id)).toContain(
      "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
    );
    expect(selected.expectedResultCategories).toContain("Moderated Mediation");
  });

  it("routes common-factor models to CB-SEM point or recursive case bootstrap", () => {
    const strict = context(model([["x", "y"]], "cbsem_common_factor"));

    const point = resolveUnifiedSemCalculationV1({
      method: "cbsem",
      context: strict,
      cbsemInference: "point",
      bootstrap,
    });
    const inference = resolveUnifiedSemCalculationV1({
      method: "cbsem",
      context: strict,
      cbsemInference: "case_bootstrap",
      bootstrap,
    });

    expect(point).toMatchObject({ route: "general_sem_cbsem", inference: "point" });
    expect(point.capabilityCells.map((cell) => cell.cell_id)).toEqual(["qpls3.cbsem.general_sem_ml"]);
    expect(inference.capabilityCells.map((cell) => cell.cell_id)).toEqual([
      "qpls3.cbsem.general_sem_ml",
      "qpls3.cbsem.bootstrap.recursive_sem",
    ]);
    expect(inference.expectedResultCategories).toContain("Bootstrap Inference");
  });

  it("keeps CFA case bootstrap on its existing exact compatibility cell", () => {
    const cfa = convertLegacyBasicModelV4({
      id: "model:unified-cfa",
      name: "Unified CFA",
      constructs: ["x", "y"].map((id) => ({
        id,
        name: id.toUpperCase(),
        short_name: id.toUpperCase(),
        mode: "reflective" as const,
        indicators: [`${id}1`, `${id}2`, `${id}3`],
      })),
      paths: [],
    }, "cbsem_common_factor");

    const inference = resolveUnifiedSemCalculationV1({
      method: "cbsem",
      context: context(cfa),
      cbsemInference: "case_bootstrap",
      bootstrap,
    });

    expect(inference).toMatchObject({
      route: "exact_cbsem_compatibility",
      controllerPreflightRequired: true,
      canStart: true,
    });
    expect(inference.capabilityCells.map((cell) => cell.cell_id)).toEqual([
      "qpls3.cbsem.ml",
      "qpls3.cbsem.bootstrap",
    ]);
  });

  it("does not misclassify control regressions as a path-free CFA", () => {
    const factorModel = model([["x", "y"]], "cbsem_common_factor");
    const relation = factorModel.relations.find((candidate) => candidate.kind === "structural");
    if (!relation || relation.kind !== "structural") throw new Error("Missing structural relation");
    relation.role = "control";

    const plan = resolveUnifiedSemCalculationV1({
      method: "cbsem",
      context: context(factorModel),
      cbsemInference: "case_bootstrap",
      bootstrap,
    });

    expect(plan.inventory).toMatchObject({ structuralRelationCount: 0, structuralRegressionCount: 1 });
    expect(plan.route).toBe("general_sem_cbsem");
  });

  it("combines strict model and reopened canonical inventories without changing either input", () => {
    const strict = context(model([["x", "m"], ["m", "y"]]));
    const document = {
      general_sem_results: {
        specific_indirect_effects: [{}, {}],
        interaction_effects: [{}],
        conditional_effects: [{}, {}, {}],
        higher_order_stages: [{}, {}],
        conditional_indirect_effects: [{}, {}, {}],
        moderated_mediation_indices: [{}],
        cbsem_parameters: [{}, {}],
        cbsem_fit: [{}],
        cbsem_bootstrap_inference: [{}, {}, {}],
      },
    } as unknown as CanonicalResultDocumentV2;
    const withDocument = { ...strict, canonicalDocument: document };
    const before = structuredClone(withDocument);

    const inventory = detectUnifiedSemFeatureInventoryV1(withDocument);

    expect(inventory.resultFamilies).toEqual({
      mediationRows: 2,
      moderationRows: 4,
      higherOrderStages: 2,
      moderatedMediationRows: 4,
      cbsemParameterRows: 2,
      cbsemFitRows: 1,
      bootstrapInferenceRows: 3,
    });
    expect(withDocument).toEqual(before);
  });
});
