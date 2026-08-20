import { describe, expect, it } from "vitest";
import { capabilityRegistryV2 } from "./capabilityRegistryV2";
import { defaultGeneralSemConfigV1, type GeneralSemConfigV1 } from "./generalSemConfigV1";
import {
  buildGeneralSemModeratedMediationSelectionV1,
  GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1,
} from "./generalSemModeratedMediationAuthoringV1";
import {
  canonicalizeSemModelV4,
  convertLegacyBasicModelV4,
  type SemModelV4,
} from "./semModelV4";

function config(): GeneralSemConfigV1 {
  const value = defaultGeneralSemConfigV1();
  value.inference = {
    kind: "case_bootstrap",
    resamples: 500,
    seed: 42,
    confidence_level: 0.95,
    interval: "percentile",
    tail: "two_sided",
  };
  return value;
}

function addInteraction(
  model: SemModelV4,
  focalSource: string,
  moderator: string,
  focalTarget: string,
): void {
  const focal = model.relations.find((relation) => relation.kind === "structural"
    && relation.source === focalSource && relation.target === focalTarget);
  if (!focal) throw new Error("Missing focal relation");
  model.variables.push({ kind: "derived", id: "derived:interaction", label: "Interaction" });
  model.relations.push({
    kind: "structural",
    id: "relation:interaction:effect",
    source: "derived:interaction",
    target: focalTarget,
    parameter: "parameter:interaction:effect",
    intercept_parameter: null,
  });
  model.parameters.push({
    kind: "free",
    id: "parameter:interaction:effect",
    label: "Interaction effect",
    target: { kind: "regression", source: "derived:interaction", target: focalTarget },
    group_overrides: [],
  });
  model.derived_terms.push({
    kind: "interaction_v2",
    id: "interaction:bounded",
    output: "derived:interaction",
    operands: [focalSource, moderator],
    focal_relation: focal.id,
    method: "two_stage",
    hierarchy_policy: "strong",
    product_indicator: null,
  });
}

function model(
  stage: "first_stage" | "second_stage",
  multipleCandidates = false,
): SemModelV4 {
  const paths: Array<[string, string]> = stage === "first_stage"
    ? [["x", "m"], ["m", "y"], ["x", "y"], ["w", "m"]]
    : multipleCandidates
      ? [["x", "m"], ["z", "m"], ["m", "y"], ["w", "y"]]
      : [["x", "m"], ["m", "y"], ["x", "y"], ["w", "y"]];
  const ids = [...new Set(paths.flat())].sort();
  const value = convertLegacyBasicModelV4({
    id: `model:${stage}`,
    name: stage,
    constructs: ids.map((id) => ({
      id,
      name: id.toUpperCase(),
      short_name: id.toUpperCase(),
      mode: "reflective",
      indicators: [`${id}1`, `${id}2`],
    })),
    paths: paths.map(([source, target]) => ({ source, target })),
  }, "pls_composite");
  addInteraction(
    value,
    stage === "first_stage" ? "construct:x" : "construct:m",
    "construct:w",
    stage === "first_stage" ? "construct:m" : "construct:y",
  );
  return canonicalizeSemModelV4(value);
}

describe("General SEM moderated-mediation authoring v1", () => {
  it("auto-selects the sole first-stage path and freezes the exact five-target inventory", () => {
    const selection = buildGeneralSemModeratedMediationSelectionV1({
      model: model("first_stage"),
      config: config(),
    });

    expect(selection.status).toBe("ready");
    if (selection.status !== "ready") throw new Error("Expected ready selection");
    expect(selection.autoSelected).toBe(true);
    expect(selection.selectedPath).toMatchObject({
      moderatedStage: "first_stage",
      xId: "construct:x",
      mediatorId: "construct:m",
      yId: "construct:y",
      moderatorId: "construct:w",
    });
    expect(selection.targetInventory.map((target) => target.kind)).toEqual([
      "scientific_gamma",
      "conditional_indirect",
      "conditional_indirect",
      "conditional_indirect",
      "index_of_moderated_mediation",
    ]);
    expect(selection.targetInventory.flatMap((target) => (
      target.kind === "conditional_indirect" ? [target.moderatorValue] : []
    ))).toEqual([-1, 0, 1]);
    expect(selection.revisedConfig.requested_effect_estimands).toEqual([{
      kind: "specific_path",
      estimand_id: selection.selectedPath.estimandId,
      ordered_relation_ids: [...selection.selectedPath.orderedRelationIds],
    }]);
  });

  it("requires an explicit stable path when the second-stage interaction has multiple candidates", () => {
    const strictModel = model("second_stage", true);
    const unresolved = buildGeneralSemModeratedMediationSelectionV1({
      model: strictModel,
      config: config(),
    });

    expect(unresolved.status).toBe("blocked");
    expect(unresolved.candidates).toHaveLength(2);
    expect(unresolved.issues.map((item) => item.code)).toContain(
      "general_sem.moderated_mediation.path_selection_required",
    );

    const chosen = buildGeneralSemModeratedMediationSelectionV1({
      model: strictModel,
      config: config(),
      selectedPathId: unresolved.candidates[1]!.pathId,
    });
    expect(chosen.status).toBe("ready");
    expect(chosen.selectedPath?.moderatedStage).toBe("second_stage");
    expect(chosen.autoSelected).toBe(false);
  });

  it("preserves candidate mapping under declaration reorderings", () => {
    const original = model("second_stage", true);
    const reordered = structuredClone(original);
    reordered.variables.reverse();
    reordered.relations.reverse();
    reordered.parameters.reverse();

    const expected = buildGeneralSemModeratedMediationSelectionV1({ model: original, config: config() });
    const actual = buildGeneralSemModeratedMediationSelectionV1({ model: reordered, config: config() });

    expect(actual.candidates).toEqual(expected.candidates);
  });

  it("fails closed for missing bootstrap and conflicting effect requests", () => {
    const noBootstrap = buildGeneralSemModeratedMediationSelectionV1({
      model: model("first_stage"),
      config: defaultGeneralSemConfigV1(),
    });
    expect(noBootstrap.status).toBe("blocked");
    expect(noBootstrap.issues.map((item) => item.code)).toContain(
      "general_sem.moderated_mediation.bootstrap_required",
    );

    const conflicting = config();
    conflicting.requested_effect_estimands = [{
      kind: "total_effect",
      estimand_id: "effect:x:y",
      source_id: "construct:x",
      target_id: "construct:y",
    }];
    const blocked = buildGeneralSemModeratedMediationSelectionV1({
      model: model("first_stage"),
      config: conflicting,
    });
    expect(blocked.status).toBe("blocked");
    expect(blocked.issues.map((item) => item.code)).toContain(
      "general_sem.moderated_mediation.effect_requests_conflict",
    );
  });

  it("binds the supplemental cell to the exact opt-in Labs Registry authority", () => {
    expect(GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1).toEqual({
      registry_schema_version: 2,
      capability_id: "smartpls.mediation",
      cell_id: "qpls3.pls.general_sem_two_way_moderated_mediation_bootstrap",
      capability_version: "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1",
    });
    const matches = capabilityRegistryV2.quickPlsCell(
      GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1.cell_id,
    );
    expect(matches).toHaveLength(1);
    expect(matches[0]).toMatchObject({
      row: { capability_id: "smartpls.mediation" },
      cell: {
        capability_version: "general_sem_pls_two_way_moderated_mediation_full_model_case_bootstrap_v1",
        coverage_state: "partial",
        evidence_state: "engine_only",
        surface: "labs",
      },
    });
    expect(capabilityRegistryV2.availability(
      GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1.capability_id,
      GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1.cell_id,
      false,
    ).selectable).toBe(false);
    expect(capabilityRegistryV2.availability(
      GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1.capability_id,
      GENERAL_SEM_TWO_WAY_MODERATED_MEDIATION_BOOTSTRAP_CELL_V1.cell_id,
      true,
    ).selectable).toBe(true);
  });
});
