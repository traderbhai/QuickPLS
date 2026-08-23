import { describe, expect, it } from "vitest";
import { convertLegacyBasicModelV4, parseSemModelV4AuthoringDraft, validateSemModelV4, type LegacyBasicModelV4Input, type SemModelV4, type SemVariableV4 } from "./semModelV4";
import {
  GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
  GENERAL_SEM_INTERACTION_V2_EDITOR_INTENT_VERSION_V1,
  parseStandardSemModelV4AuthorityRecordV1,
  reduceStandardSemModelV4AuthorityV1,
  StandardSemModelV4AuthorityError,
  standardSemGeneralSemInteractionV2EffectRelationIdV1,
  standardSemGeneralSemInteractionV2ModeratorMainRelationIdV1,
  standardSemGeneralSemInteractionV2OutputIdV1,
  standardSemGeneralSemInteractionV2TermIdV1,
  standardSemGeneralSemModerationV3ThreeWayTermIdV1,
  standardSemFactorVarianceParameterIdV1,
  standardSemMeasurementRelationIdV1,
  type AddGeneralSemInteractionV2EditorIntentV1,
  type StandardSemModelV4AuthorityRecordV1,
} from "./standardSemModelV4Authority";

const legacy: LegacyBasicModelV4Input = {
  id: "standard-model",
  name: "Standard model",
  constructs: [
    { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
    { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
  ],
  paths: [{ source: "x", target: "y" }],
  controls: [],
  higher_order_constructs: [],
  interactions: [],
};

const authority = (model: SemModelV4 = convertLegacyBasicModelV4(legacy, "cbsem_common_factor"), digest = "a".repeat(64)): StandardSemModelV4AuthorityRecordV1 => ({
  schema_version: 1,
  model_document_sha256: digest,
  model,
});

const observed = (id: string): Extract<SemVariableV4, { kind: "observed" }> => ({
  kind: "observed",
  id,
  label: id.toUpperCase(),
  source_column: id,
  scale: "continuous",
  role: "indicator",
  categories: [],
  value_labels: {},
  missing_markers: [],
  transformation_lineage: [],
});

function importedCfaModel(): SemModelV4 {
  const importedSeed = convertLegacyBasicModelV4({
    ...legacy,
    id: "imported-cfa",
    name: "Imported CFA",
    constructs: [
      { id: "m1", name: "Mediator 1", short_name: "M1", mode: "reflective", indicators: ["m11", "m12", "m13"] },
      { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2", "x3"] },
      { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2", "y3"] },
    ],
    paths: [],
  }, "cbsem_common_factor");
  const importedVarianceIds = new Map([
    ["construct:m1", "factor_variance_6d31"],
    ["construct:x", "factor_variance_78"],
    ["construct:y", "factor_variance_79"],
  ]);
  const replacedParameterIds = new Map(importedSeed.variables.flatMap((variable) => variable.kind === "common_factor"
    ? [[variable.disturbance_policy.parameter, importedVarianceIds.get(variable.id)!] as const]
    : []));
  return parseSemModelV4AuthoringDraft({
    ...importedSeed,
    variables: importedSeed.variables.map((variable) => variable.kind === "common_factor"
      ? { ...variable, disturbance_policy: { ...variable.disturbance_policy, parameter: importedVarianceIds.get(variable.id)! } }
      : variable),
    parameters: importedSeed.parameters.map((parameter) => replacedParameterIds.has(parameter.id)
      ? { ...parameter, id: replacedParameterIds.get(parameter.id)! }
      : parameter),
  });
}

function interactionAuthority(): StandardSemModelV4AuthorityRecordV1 {
  const withModerator = reduceStandardSemModelV4AuthorityV1(authority(), {
    kind: "add_construct",
    variable_id: "construct:z",
    label: "Moderator",
    representation: { kind: "composite", weighting: { kind: "mode_a" } },
    indicators: [observed("observed:z1")],
  });
  const focal = withModerator.model.relations.find((relation) =>
    relation.kind === "structural"
    && relation.source === "construct:x"
    && relation.target === "construct:y");
  if (!focal) throw new Error("Expected the focal structural relation.");
  const withInteraction = reduceStandardSemModelV4AuthorityV1(authority(withModerator.model, "b".repeat(64)), {
    kind: "add_interaction",
    term_id: "interaction-v2:x-z",
    output_id: "derived:interaction-v2:x-z",
    label: "X by Z V2",
    predictor: "construct:x",
    moderator: "construct:z",
    focal_relation: focal.id,
    outcome: "construct:y",
    method: "two_stage",
  });
  return authority(withInteraction.model, "b".repeat(64));
}

function generalSemModerationBase(): {
  source: StandardSemModelV4AuthorityRecordV1;
  focalRelationId: string;
} {
  const withModerator = reduceStandardSemModelV4AuthorityV1(authority(), {
    kind: "add_construct",
    variable_id: "construct:z",
    label: "Moderator",
    representation: { kind: "composite", weighting: { kind: "mode_a" } },
    indicators: [observed("observed:z1")],
  });
  const focal = withModerator.model.relations.find((relation) =>
    relation.kind === "structural"
    && relation.source === "construct:x"
    && relation.target === "construct:y");
  if (!focal) throw new Error("Expected the focal structural relation.");
  return {
    source: authority(withModerator.model, "d".repeat(64)),
    focalRelationId: focal.id,
  };
}

function generalSemInteractionV2Intent(
  focalRelationId: string,
): AddGeneralSemInteractionV2EditorIntentV1 {
  return {
    kind: "add_general_sem_interaction_v2",
    intent_version: GENERAL_SEM_INTERACTION_V2_EDITOR_INTENT_VERSION_V1,
    sem_generation: "general_sem_v1",
    label: "X by Z",
    operands: ["construct:x", "construct:z"],
    focal_relation: focalRelationId,
    outcome: "construct:y",
    method: "two_stage",
    hierarchy_policy: "strong",
  };
}

function interactionV2Authority(): StandardSemModelV4AuthorityRecordV1 {
  const source = interactionAuthority();
  const model = structuredClone(source.model);
  const index = model.derived_terms.findIndex((term) => term.id === "interaction-v2:x-z");
  const term = model.derived_terms[index];
  if (term?.kind !== "interaction") throw new Error("Expected the legacy interaction seed.");
  model.derived_terms[index] = {
    kind: "interaction_v2",
    id: term.id,
    output: term.output,
    operands: [term.predictor, term.moderator],
    focal_relation: term.focal_relation,
    method: term.method,
    hierarchy_policy: "strong",
  };
  return authority(parseSemModelV4AuthoringDraft(model), "c".repeat(64));
}

describe("StandardSemModelV4 authority", () => {
  it("strictly parses and freezes the exact authority revision", () => {
    const parsed = parseStandardSemModelV4AuthorityRecordV1(authority());
    expect(Object.isFrozen(parsed)).toBe(true);
    expect(Object.isFrozen(parsed.model.variables)).toBe(true);
    expect(() => parseStandardSemModelV4AuthorityRecordV1({ ...authority(), model_document_sha256: ` ${"a".repeat(64)}` }))
      .toThrowError(expect.objectContaining({ code: "standard_sem_authority.digest_invalid" }));
    expect(() => parseStandardSemModelV4AuthorityRecordV1({ ...authority(), extra: true }))
      .toThrowError(expect.objectContaining({ code: "standard_sem_authority.field_unknown" }));
  });

  it("adds one construct and its indicators atomically without mutating authority", () => {
    const source = authority();
    const before = JSON.stringify(source);
    const candidate = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_construct",
      variable_id: "construct:z",
      label: " Z construct ",
      representation: { kind: "composite", weighting: { kind: "mode_b" } },
      indicators: [observed("observed:z1")],
    });
    expect(JSON.stringify(source)).toBe(before);
    expect(candidate.expected_model_document_sha256).toBe(source.model_document_sha256);
    expect(candidate.model.variables).toContainEqual(expect.objectContaining({ id: "construct:z", label: "Z construct", kind: "composite" }));
    expect(candidate.model.relations).toContainEqual(expect.objectContaining({ id: standardSemMeasurementRelationIdV1("construct:z", "observed:z1"), kind: "measurement_causal" }));
    expect(Object.isFrozen(candidate.model)).toBe(true);
  });

  it("rejects identity normalization and ambiguous common-factor construction", () => {
    expect(() => reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_construct",
      variable_id: " construct:z ",
      label: "Z",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [],
    })).toThrowError(expect.objectContaining({ code: "standard_sem_authority.stable_id_invalid" }));

    expect(() => reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_construct",
      variable_id: "construct:z",
      label: "Z",
      representation: { kind: "common_factor", identification: { kind: "marker_loading", indicator: "observed:missing" } },
      indicators: [observed("observed:z1")],
    })).toThrowError(StandardSemModelV4AuthorityError);
  });

  it("supports fixed-variance factor creation in one candidate", () => {
    const candidate = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_construct",
      variable_id: "construct:z",
      label: "Z",
      representation: { kind: "common_factor", identification: { kind: "fixed_variance" } },
      indicators: [observed("observed:z1"), observed("observed:z2")],
    });
    expect(candidate.model.variables).toContainEqual(expect.objectContaining({ id: "construct:z", identification: { kind: "fixed_variance" } }));
    expect(candidate.model.parameters).toContainEqual(expect.objectContaining({ kind: "fixed", value: 1 }));
  });

  it("preserves imported factor-variance identities across Advanced Parameter equality edits", () => {
    const imported = importedCfaModel();
    const factorVarianceIds = new Map(imported.variables.flatMap((variable) => variable.kind === "common_factor"
      ? [[variable.id, variable.disturbance_policy.parameter] as const]
      : []));
    expect([...factorVarianceIds.values()]).toEqual(["factor_variance_6d31", "factor_variance_78", "factor_variance_79"]);
    expect(imported.parameters.some((parameter) => parameter.id.startsWith("standard:v1:factor-variance:"))).toBe(false);

    const loadingIds = imported.parameters
      .filter((parameter) => parameter.kind === "free" && parameter.target.kind === "loading" && parameter.target.construct === "construct:m1")
      .map((parameter) => parameter.id);
    expect(loadingIds).toHaveLength(2);
    const first = reduceStandardSemModelV4AuthorityV1(authority(imported), {
      kind: "set_parameter_specification",
      parameter_id: loadingIds[0],
      specification: { kind: "free", start: 0.7, lower: null, upper: null, equality_label: "V255Evidence" },
    });
    const second = reduceStandardSemModelV4AuthorityV1(authority(first.model, "b".repeat(64)), {
      kind: "set_parameter_specification",
      parameter_id: loadingIds[1],
      specification: { kind: "free", start: 0.7, lower: null, upper: null, equality_label: "V255Evidence" },
    });

    expect(second.model.parameters).toHaveLength(imported.parameters.length);
    expect(second.model.parameters.some((parameter) => parameter.id.startsWith("standard:v1:factor-variance:"))).toBe(false);
    expect(second.model.variables.flatMap((variable) => variable.kind === "common_factor"
      ? [[variable.id, variable.disturbance_policy.parameter] as const]
      : [])).toEqual([...factorVarianceIds.entries()]);
    for (const parameterId of factorVarianceIds.values()) {
      const before = imported.parameters.find((parameter) => parameter.id === parameterId)!;
      const after = second.model.parameters.find((parameter) => parameter.id === parameterId)!;
      const { label: _beforeLabel, ...beforeAuthority } = before;
      const { label: _afterLabel, ...afterAuthority } = after;
      expect(afterAuthority).toEqual(beforeAuthority);
    }
    expect(second.model.parameters.filter((parameter) => loadingIds.includes(parameter.id)))
      .toEqual(expect.arrayContaining(loadingIds.map((id) => expect.objectContaining({ id, kind: "free", equality_label: "V255Evidence" }))));
    expect(validateSemModelV4(second.model)).toEqual([]);
  });

  it("retargets an imported factor variance without changing its identity when endogeneity changes", () => {
    const source = authority();
    const structural = source.model.relations.find((relation) => relation.kind === "structural");
    if (!structural || structural.kind !== "structural") throw new Error("Expected an imported structural relationship.");
    const outcome = source.model.variables.find((variable) => variable.id === structural.target);
    if (!outcome || outcome.kind !== "common_factor") throw new Error("Expected an imported common-factor outcome.");
    const parameterId = outcome.disturbance_policy.parameter;

    const removed = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "delete_relationship",
      relationship_id: structural.id,
    });
    expect(removed.model.variables).toContainEqual(expect.objectContaining({
      id: outcome.id,
      disturbance_policy: { kind: "exogenous_variance", parameter: parameterId },
    }));
    expect(removed.model.parameters).toContainEqual(expect.objectContaining({
      id: parameterId,
      target: { kind: "variance", endpoint: { kind: "variable", id: outcome.id } },
    }));

    const restored = reduceStandardSemModelV4AuthorityV1(authority(removed.model, "b".repeat(64)), {
      kind: "add_relationship",
      relationship_id: "relationship:restored",
      definition: { kind: "structural", source: structural.source, target: structural.target, label: "Restored path" },
    });
    expect(restored.model.variables).toContainEqual(expect.objectContaining({
      id: outcome.id,
      disturbance_policy: { kind: "endogenous_disturbance", parameter: parameterId },
    }));
    expect(restored.model.parameters).toContainEqual(expect.objectContaining({
      id: parameterId,
      target: { kind: "variance", endpoint: { kind: "disturbance_of", id: outcome.id } },
    }));
    expect(restored.model.parameters.some((parameter) => parameter.id.startsWith("standard:v1:factor-variance:"))).toBe(false);
    expect(validateSemModelV4(restored.model)).toEqual([]);
  });

  it("keeps an imported variance identity through fixed-variance identification and parameter restore", () => {
    const imported = importedCfaModel();
    const factor = imported.variables.find((variable) => variable.id === "construct:m1");
    if (!factor || factor.kind !== "common_factor" || factor.identification.kind !== "marker_loading") {
      throw new Error("Expected the imported marker-identified factor.");
    }
    const parameterId = factor.disturbance_policy.parameter;
    const marker = factor.identification.indicator;

    const fixed = reduceStandardSemModelV4AuthorityV1(authority(imported), {
      kind: "set_factor_identification",
      variable_id: factor.id,
      identification: { kind: "fixed_variance" },
    });
    expect(fixed.model.variables).toContainEqual(expect.objectContaining({
      id: factor.id,
      identification: { kind: "fixed_variance" },
      disturbance_policy: { kind: "exogenous_variance", parameter: parameterId },
    }));
    expect(fixed.model.parameters).toContainEqual(expect.objectContaining({
      id: parameterId,
      kind: "fixed",
      value: 1,
    }));

    const markerRestored = reduceStandardSemModelV4AuthorityV1(authority(fixed.model, "b".repeat(64)), {
      kind: "set_factor_identification",
      variable_id: factor.id,
      identification: { kind: "marker_loading", indicator: marker },
    });
    expect(markerRestored.model.variables).toContainEqual(expect.objectContaining({
      id: factor.id,
      identification: { kind: "marker_loading", indicator: marker },
      disturbance_policy: { kind: "exogenous_variance", parameter: parameterId },
    }));
    expect(markerRestored.model.parameters).toContainEqual(expect.objectContaining({
      id: parameterId,
      kind: "free",
      start: 1,
      lower: 0,
    }));

    const overridden = reduceStandardSemModelV4AuthorityV1(authority(markerRestored.model, "c".repeat(64)), {
      kind: "set_parameter_specification",
      parameter_id: parameterId,
      specification: { kind: "fixed", value: 2 },
    });
    const parameterRestored = reduceStandardSemModelV4AuthorityV1(authority(overridden.model, "d".repeat(64)), {
      kind: "restore_parameter",
      parameter_id: parameterId,
    });
    expect(parameterRestored.model.parameters).toContainEqual(expect.objectContaining({
      id: parameterId,
      kind: "free",
    }));
    expect(parameterRestored.model.variables).toContainEqual(expect.objectContaining({
      id: factor.id,
      disturbance_policy: { kind: "exogenous_variance", parameter: parameterId },
    }));
    expect(parameterRestored.model.parameters.some((parameter) => parameter.id.startsWith("standard:v1:factor-variance:"))).toBe(false);
    expect(validateSemModelV4(parameterRestored.model)).toEqual([]);
  });

  it("removes an imported factor variance when converting to a composite and creates a new identity only when restored", () => {
    const imported = importedCfaModel();
    const factor = imported.variables.find((variable) => variable.id === "construct:m1");
    if (!factor || factor.kind !== "common_factor" || factor.identification.kind !== "marker_loading") {
      throw new Error("Expected the imported marker-identified factor.");
    }
    const importedParameterId = factor.disturbance_policy.parameter;
    const standardParameterId = standardSemFactorVarianceParameterIdV1(factor.id);
    const importedParameter = imported.parameters.find((parameter) => parameter.id === importedParameterId)!;
    const duplicated = parseSemModelV4AuthoringDraft({
      ...imported,
      parameters: [
        ...imported.parameters,
        { ...importedParameter, id: standardParameterId, label: "Duplicated factor variance" },
      ],
    });

    const composite = reduceStandardSemModelV4AuthorityV1(authority(duplicated), {
      kind: "set_construct_representation",
      variable_id: factor.id,
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
    });
    expect(composite.model.variables).toContainEqual(expect.objectContaining({ id: factor.id, kind: "composite" }));
    expect(composite.model.parameters.some((parameter) => parameter.id === importedParameterId)).toBe(false);
    expect(composite.model.parameters.some((parameter) => parameter.id === standardParameterId)).toBe(false);
    expect(composite.model.parameters.some((parameter) => parameter.target.kind === "variance"
      && parameter.target.endpoint.kind !== "residual_of"
      && parameter.target.endpoint.id === factor.id)).toBe(false);
    expect(validateSemModelV4(composite.model)).toEqual([]);

    const restored = reduceStandardSemModelV4AuthorityV1(authority(composite.model, "b".repeat(64)), {
      kind: "set_construct_representation",
      variable_id: factor.id,
      representation: { kind: "common_factor", identification: { kind: "marker_loading", indicator: factor.identification.indicator } },
    });
    const restoredParameterId = standardParameterId;
    expect(restored.model.variables).toContainEqual(expect.objectContaining({
      id: factor.id,
      kind: "common_factor",
      disturbance_policy: { kind: "exogenous_variance", parameter: restoredParameterId },
    }));
    expect(restored.model.parameters.some((parameter) => parameter.id === importedParameterId)).toBe(false);
    expect(restored.model.parameters.filter((parameter) => parameter.id === restoredParameterId)).toHaveLength(1);
    expect(validateSemModelV4(restored.model)).toEqual([]);
  });

  it("preserves a valid fixed-zero disturbance across unrelated edits and retargets it in place with endogeneity", () => {
    const imported = importedCfaModel();
    const factor = imported.variables.find((variable) => variable.id === "construct:m1");
    if (!factor || factor.kind !== "common_factor") throw new Error("Expected the imported common factor.");
    const parameterId = factor.disturbance_policy.parameter;
    const fixedZeroToleranceBoundary = 1e-12;
    const fixedZero = parseSemModelV4AuthoringDraft({
      ...imported,
      variables: imported.variables.map((variable) => variable.id === factor.id && variable.kind === "common_factor"
        ? { ...variable, disturbance_policy: { kind: "fixed_zero", parameter: parameterId } }
        : variable),
      parameters: imported.parameters.map((parameter) => parameter.id === parameterId
        ? { kind: "fixed", id: parameter.id, label: parameter.label, target: parameter.target, value: fixedZeroToleranceBoundary, group_overrides: parameter.group_overrides ?? [] }
        : parameter),
    });
    expect(validateSemModelV4(fixedZero)).toEqual([]);

    const unrelated = reduceStandardSemModelV4AuthorityV1(authority(fixedZero), {
      kind: "set_model_name",
      name: "Imported CFA renamed",
    });
    expect(unrelated.model.variables).toContainEqual(expect.objectContaining({
      id: factor.id,
      disturbance_policy: { kind: "fixed_zero", parameter: parameterId },
    }));
    expect(unrelated.model.parameters.find((parameter) => parameter.id === parameterId))
      .toEqual(fixedZero.parameters.find((parameter) => parameter.id === parameterId));

    const endogenous = reduceStandardSemModelV4AuthorityV1(authority(unrelated.model, "b".repeat(64)), {
      kind: "add_relationship",
      relationship_id: "relationship:x-m1",
      definition: { kind: "structural", source: "construct:x", target: factor.id, label: "Predictor to mediator" },
    });
    expect(endogenous.model.variables).toContainEqual(expect.objectContaining({
      id: factor.id,
      disturbance_policy: { kind: "fixed_zero", parameter: parameterId },
    }));
    expect(endogenous.model.parameters).toContainEqual(expect.objectContaining({
      id: parameterId,
      kind: "fixed",
      value: fixedZeroToleranceBoundary,
      target: { kind: "variance", endpoint: { kind: "disturbance_of", id: factor.id } },
    }));
    expect(validateSemModelV4(endogenous.model)).toEqual([]);

    const exogenous = reduceStandardSemModelV4AuthorityV1(authority(endogenous.model, "c".repeat(64)), {
      kind: "delete_relationship",
      relationship_id: "relationship:x-m1",
    });
    expect(exogenous.model.variables).toContainEqual(expect.objectContaining({
      id: factor.id,
      disturbance_policy: { kind: "fixed_zero", parameter: parameterId },
    }));
    expect(exogenous.model.parameters).toContainEqual(expect.objectContaining({
      id: parameterId,
      kind: "fixed",
      value: fixedZeroToleranceBoundary,
      target: { kind: "variance", endpoint: { kind: "variable", id: factor.id } },
    }));
    expect(exogenous.model.parameters.some((parameter) => parameter.id.startsWith("standard:v1:factor-variance:"))).toBe(false);
    expect(validateSemModelV4(exogenous.model)).toEqual([]);
  });

  it("fails closed when a new factor's generated variance identity already belongs to another parameter", () => {
    const source = importedCfaModel();
    const collisionId = standardSemFactorVarianceParameterIdV1("construct:z");
    const loading = source.relations.find((relation) => relation.kind === "measurement_effect");
    if (!loading || loading.kind !== "measurement_effect") throw new Error("Expected an imported loading.");
    const collisionSource = parseSemModelV4AuthoringDraft({
      ...source,
      relations: source.relations.map((relation) => relation.id === loading.id
        ? { ...relation, parameter: collisionId }
        : relation),
      parameters: source.parameters.map((parameter) => parameter.id === loading.parameter
        ? { ...parameter, id: collisionId }
        : parameter),
    });
    expect(validateSemModelV4(collisionSource)).toEqual([]);
    const before = JSON.stringify(collisionSource);

    expect(() => reduceStandardSemModelV4AuthorityV1(authority(collisionSource), {
      kind: "add_construct",
      variable_id: "construct:z",
      label: "Collision factor",
      representation: { kind: "common_factor", identification: { kind: "marker_loading", indicator: "observed:z1" } },
      indicators: [observed("observed:z1"), observed("observed:z2")],
    })).toThrowError(expect.objectContaining({
      code: "standard_sem_authority.identity_duplicate",
      subject: collisionId,
    }));
    expect(JSON.stringify(collisionSource)).toBe(before);
    expect(collisionSource.parameters.find((parameter) => parameter.id === collisionId)?.target.kind).toBe("loading");
  });

  it("replaces a HOC atomically while preserving its term, output, and structural relationships", () => {
    const source = authority(convertLegacyBasicModelV4(legacy, "pls_composite"));
    const added = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_higher_order",
      term_id: "term:hoc",
      output_id: "derived:hoc",
      label: "Corporate standing",
      components: ["construct:x", "construct:y"],
      approach: "embedded_two_stage",
      measurement_type: "reflective_reflective",
    });
    const relationBytes = JSON.stringify(added.model.relations);
    const replaced = reduceStandardSemModelV4AuthorityV1(authority(added.model, "b".repeat(64)), {
      kind: "replace_higher_order",
      term_id: "term:hoc",
      output_id: "derived:hoc",
      label: "Corporate standing revised",
      components: ["construct:x", "construct:y"],
      approach: "embedded_two_stage",
      measurement_type: "reflective_formative",
    });

    expect(replaced.model.derived_terms).toContainEqual(expect.objectContaining({
      kind: "higher_order",
      id: "term:hoc",
      output: "derived:hoc",
      approach: "embedded_two_stage",
      measurement_type: "reflective_formative",
    }));
    expect(replaced.model.variables).toContainEqual(expect.objectContaining({
      kind: "derived",
      id: "derived:hoc",
      label: "Corporate standing revised",
    }));
    expect(JSON.stringify(replaced.model.relations)).toBe(relationBytes);
  });

  it("removes only the exact HOC identity and preserves its components and unrelated paths", () => {
    const source = authority(convertLegacyBasicModelV4(legacy, "pls_composite"));
    const added = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_higher_order",
      term_id: "term:hoc",
      output_id: "derived:hoc",
      label: "Corporate standing",
      components: ["construct:x", "construct:y"],
      approach: "embedded_two_stage",
      measurement_type: "reflective_reflective",
    });
    const relationsBefore = structuredClone(added.model.relations);
    const removed = reduceStandardSemModelV4AuthorityV1(authority(added.model, "b".repeat(64)), {
      kind: "remove_higher_order",
      term_id: "term:hoc",
      output_id: "derived:hoc",
    });

    expect(removed.model.variables.some((variable) => variable.id === "derived:hoc")).toBe(false);
    expect(removed.model.derived_terms.some((term) => term.id === "term:hoc")).toBe(false);
    expect(removed.model.variables).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: "construct:x" }),
      expect.objectContaining({ id: "construct:y" }),
    ]));
    expect(removed.model.relations).toEqual(relationsBefore);
    expect(() => reduceStandardSemModelV4AuthorityV1(authority(added.model, "b".repeat(64)), {
      kind: "remove_higher_order",
      term_id: "term:hoc",
      output_id: "derived:stale",
    })).toThrowError(expect.objectContaining({ code: "standard_sem_authority.higher_order_identity_mismatch" }));
  });

  it("strictly replaces the complete canonical document without mutating the source authority", () => {
    const source = authority();
    const sourceJson = JSON.stringify(source);
    const replacement = structuredClone(source.model);
    replacement.name = "Complete expert replacement";
    replacement.group = { kind: "single_group" };
    const strictReplacement = parseSemModelV4AuthoringDraft(replacement);

    const candidate = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "replace_complete_model",
      model: replacement,
    });

    expect(JSON.stringify(source)).toBe(sourceJson);
    expect(candidate.expected_model_document_sha256).toBe(source.model_document_sha256);
    expect(candidate.model).toEqual(expect.objectContaining({
      id: source.model.id,
      name: "Complete expert replacement",
      group: { kind: "single_group" },
      data_binding: replacement.data_binding,
    }));
    expect(candidate.model.variables).toEqual(strictReplacement.variables);
    expect(candidate.model.relations).toEqual(strictReplacement.relations);
    expect(candidate.model.parameters).toEqual(strictReplacement.parameters);
    expect(candidate.model.constraints).toEqual(strictReplacement.constraints);
    expect(candidate.model.derived_terms).toEqual(strictReplacement.derived_terms);
    expect(candidate.model.annotations).toEqual(strictReplacement.annotations);
    expect(candidate.model.presentation).toEqual(strictReplacement.presentation);
  });

  it("rejects complete-document identity changes and unknown canonical fields", () => {
    expect(() => reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "replace_complete_model",
      model: { ...authority().model, id: "another-model" },
    })).toThrowError(expect.objectContaining({ code: "standard_sem_authority.model_identity_mismatch" }));

    expect(() => reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "replace_complete_model",
      model: { ...authority().model, unsupported: true } as unknown as SemModelV4,
    })).toThrowError(expect.objectContaining({ code: "standard_sem_authority.candidate_invalid" }));
  });

  it("keeps dataset identity and both presentation-owned lanes outside complete scientific replacement", () => {
    expect(() => reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "replace_complete_model",
      model: {
        ...authority().model,
        data_binding: { ...authority().model.data_binding, dataset_id: "dataset:other" },
      },
    })).toThrowError(expect.objectContaining({
      code: "standard_sem_authority.dataset_binding_switch_requires_descriptor_transaction",
      corrective_action: expect.stringContaining("descriptor-aware project transaction"),
    }));

    expect(() => reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "replace_complete_model",
      model: {
        ...authority().model,
        annotations: [{ kind: "caption", id: "caption:expert", text: "Not owned here" }],
      },
    })).toThrowError(expect.objectContaining({
      code: "standard_sem_authority.presentation_annotations_owned_by_layout",
      corrective_action: expect.stringContaining("canvas presentation controls"),
    }));

    expect(() => reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "replace_complete_model",
      model: {
        ...authority().model,
        presentation: { kind: "canvas", nodes: [], edges: [], shapes: [], images: [], lines: [] },
      },
    })).toThrowError(expect.objectContaining({
      code: "standard_sem_authority.presentation_owned_by_layout",
      corrective_action: expect.stringContaining("canvas presentation layer"),
    }));
  });

  it("authors observed and latent controls with an explicit canonical relation role", () => {
    const withControl = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_observed_variable",
      variable: { ...observed("observed:control"), role: "control" },
    });
    const candidate = reduceStandardSemModelV4AuthorityV1(authority(withControl.model, "b".repeat(64)), {
      kind: "add_relationship",
      relationship_id: "control:c-y",
      definition: { kind: "control", source: "observed:control", target: "construct:y", label: "Control C" },
    });
    expect(candidate.model.relations).toContainEqual(expect.objectContaining({ kind: "structural", id: "control:c-y", source: "observed:control", target: "construct:y", role: "control" }));

    const structural = authority().model.relations.find((relation) => relation.kind === "structural");
    if (!structural || structural.kind !== "structural") throw new Error("Expected a structural relationship.");
    const latent = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "replace_relationship",
      relationship_id: structural.id,
      definition: { kind: "control", source: structural.source, target: structural.target, label: "Latent control" },
    });
    expect(latent.model.relations).toContainEqual(expect.objectContaining({ kind: "structural", id: structural.id, source: "construct:x", target: "construct:y", role: "control" }));

    const withStructuralObserved = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "set_observed_role",
      variable_id: "observed:x1",
      role: "both",
    });
    expect(() => reduceStandardSemModelV4AuthorityV1(authority(withStructuralObserved.model, "c".repeat(64)), {
      kind: "add_relationship",
      relationship_id: "control:invalid-observed-y",
      definition: { kind: "control", source: "observed:x1", target: "construct:y", label: "Invalid observed control" },
    })).toThrowError(expect.objectContaining({ code: "standard_sem_authority.control_role_required" }));

    const observedStructural = reduceStandardSemModelV4AuthorityV1(authority(withStructuralObserved.model, "c".repeat(64)), {
      kind: "add_relationship",
      relationship_id: "structural:x1-y",
      definition: { kind: "structural", source: "observed:x1", target: "construct:y", label: "Observed predictor" },
    });
    expect(() => reduceStandardSemModelV4AuthorityV1(authority(observedStructural.model, "d".repeat(64)), {
      kind: "replace_complete_model",
      model: {
        ...observedStructural.model,
        relations: observedStructural.model.relations.map((relation) => relation.id === "structural:x1-y"
          ? { ...relation, role: "control" as const }
          : relation),
      },
    })).toThrowError(expect.objectContaining({ code: "standard_sem_authority.control_role_required" }));
  });

  it("adds a cross-loading without moving or duplicating the observed variable", () => {
    const candidate = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_cross_loading",
      construct_id: "construct:y",
      observed_id: "observed:x1",
    });
    const x1Effects = candidate.model.relations.filter((relation): relation is Extract<SemModelV4["relations"][number], { kind: "measurement_effect" }> => relation.kind === "measurement_effect" && relation.indicator === "observed:x1");
    expect(x1Effects.map((relation) => relation.construct)).toEqual(["construct:x", "construct:y"]);
    expect(candidate.model.variables.filter((variable) => variable.id === "observed:x1")).toHaveLength(1);
  });

  it("authors product-indicator interactions and polynomial terms with explicit construction", () => {
    const withModerator = reduceStandardSemModelV4AuthorityV1(authority(), {
      kind: "add_construct",
      variable_id: "construct:z",
      label: "Moderator",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [observed("observed:z1")],
    });
    const focal = withModerator.model.relations.find((relation) => relation.kind === "structural" && relation.source === "construct:x" && relation.target === "construct:y");
    if (!focal) throw new Error("Expected the focal structural relation.");
    const interaction = reduceStandardSemModelV4AuthorityV1(authority(withModerator.model, "b".repeat(64)), {
      kind: "add_interaction",
      term_id: "interaction:x-z",
      output_id: "derived:x-z",
      label: "X by Z",
      predictor: "construct:x",
      moderator: "construct:z",
      focal_relation: focal.id,
      outcome: "construct:y",
      method: "product_indicator",
      product_indicator: { centering: "double_mean_center", standardization: "sample_standard_deviation", pairing: "all_pairs" },
    });
    expect(interaction.model.derived_terms).toContainEqual(expect.objectContaining({
      kind: "interaction",
      id: "interaction:x-z",
      method: "product_indicator",
      product_indicator: { centering: "double_mean_center", standardization: "sample_standard_deviation", pairing: "all_pairs" },
    }));

    const polynomial = reduceStandardSemModelV4AuthorityV1(authority(interaction.model, "c".repeat(64)), {
      kind: "add_polynomial",
      term_id: "polynomial:x2",
      output_id: "derived:x2",
      label: "X squared",
      source: "construct:x",
      degree: 2,
    });
    expect(polynomial.model.derived_terms).toContainEqual(expect.objectContaining({ kind: "polynomial", id: "polynomial:x2", source: "construct:x", degree: 2 }));
  });

  it("authors the exact versioned General SEM interaction_v2 atomically with deterministic hierarchy paths", () => {
    const { source, focalRelationId } = generalSemModerationBase();
    const intent = generalSemInteractionV2Intent(focalRelationId);
    const before = JSON.stringify(source);
    const first = reduceStandardSemModelV4AuthorityV1(source, intent);
    const second = reduceStandardSemModelV4AuthorityV1(source, intent);
    const termId = standardSemGeneralSemInteractionV2TermIdV1(
      focalRelationId,
      "construct:x",
      "construct:z",
    );
    const outputId = standardSemGeneralSemInteractionV2OutputIdV1(termId);
    const moderatorMainId = standardSemGeneralSemInteractionV2ModeratorMainRelationIdV1(termId);
    const effectId = standardSemGeneralSemInteractionV2EffectRelationIdV1(termId);

    expect(JSON.stringify(source)).toBe(before);
    expect(first).toEqual(second);
    expect(first.readiness).toBe("ready");
    expect(first.model.derived_terms).toContainEqual({
      kind: "interaction_v2",
      id: termId,
      output: outputId,
      operands: ["construct:x", "construct:z"],
      focal_relation: focalRelationId,
      method: "two_stage",
      hierarchy_policy: "strong",
    });
    expect(first.model.derived_terms.some((term) => term.kind === "interaction")).toBe(false);
    expect(first.model.variables).toContainEqual({ kind: "derived", id: outputId, label: "X by Z" });
    expect(first.model.relations).toEqual(expect.arrayContaining([
      expect.objectContaining({ id: moderatorMainId, kind: "structural", source: "construct:z", target: "construct:y" }),
      expect.objectContaining({ id: effectId, kind: "structural", source: outputId, target: "construct:y" }),
    ]));
    expect(first.model.parameters).toEqual(expect.arrayContaining([
      expect.objectContaining({
        id: `standard:v1:relationship-parameter:${encodeURIComponent(moderatorMainId)}`,
        target: { kind: "regression", source: "construct:z", target: "construct:y" },
      }),
      expect.objectContaining({
        id: `standard:v1:relationship-parameter:${encodeURIComponent(effectId)}`,
        target: { kind: "regression", source: outputId, target: "construct:y" },
      }),
    ]));
    expect(parseSemModelV4AuthoringDraft(JSON.parse(JSON.stringify(first.model)))).toEqual(first.model);
  });

  it("rejects non-v1, non-two-stage, non-strong, and non-two-operand General SEM intents without a legacy downgrade", () => {
    const { source, focalRelationId } = generalSemModerationBase();
    const before = JSON.stringify(source);
    const valid = generalSemInteractionV2Intent(focalRelationId);
    const invalid = [
      [{ ...valid, sem_generation: "ordinary" }, "standard_sem_authority.general_sem_interaction_v2_generation_required"],
      [{ ...valid, intent_version: 2 }, "standard_sem_authority.general_sem_interaction_v2_intent_version_unsupported"],
      [{ ...valid, method: "product_indicator" }, "standard_sem_authority.general_sem_interaction_v2_method_invalid"],
      [{ ...valid, hierarchy_policy: "weak" }, "standard_sem_authority.general_sem_interaction_v2_hierarchy_invalid"],
      [{ ...valid, operands: ["construct:x"] }, "standard_sem_authority.general_sem_interaction_v2_operands_invalid"],
      [{ ...valid, operands: ["construct:x", "construct:z", "construct:y"] }, "standard_sem_authority.general_sem_interaction_v2_operands_invalid"],
    ] as const;

    for (const [intent, code] of invalid) {
      expect(() => reduceStandardSemModelV4AuthorityV1(
        source,
        intent as unknown as AddGeneralSemInteractionV2EditorIntentV1,
      )).toThrowError(expect.objectContaining({ code }));
      expect(JSON.stringify(source)).toBe(before);
      expect(source.model.derived_terms).toHaveLength(0);
    }
  });

  it("reuses an existing moderator main effect and blocks a legacy-equivalent semantic duplicate", () => {
    const { source, focalRelationId } = generalSemModerationBase();
    const withMain = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_relationship",
      relationship_id: "relation:z-y",
      definition: { kind: "structural", source: "construct:z", target: "construct:y", label: "Z to Y" },
    });
    const candidate = reduceStandardSemModelV4AuthorityV1(
      authority(withMain.model, "e".repeat(64)),
      generalSemInteractionV2Intent(focalRelationId),
    );
    expect(candidate.model.relations.filter((relation) =>
      relation.kind === "structural"
      && relation.source === "construct:z"
      && relation.target === "construct:y")).toHaveLength(1);
    expect(candidate.model.relations).toContainEqual(expect.objectContaining({ id: "relation:z-y" }));

    const legacyEquivalent = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_interaction",
      term_id: "legacy:interaction:x-z",
      output_id: "legacy:derived:x-z",
      label: "Legacy X by Z",
      predictor: "construct:x",
      moderator: "construct:z",
      focal_relation: focalRelationId,
      outcome: "construct:y",
      method: "two_stage",
    });
    expect(() => reduceStandardSemModelV4AuthorityV1(
      authority(legacyEquivalent.model, "f".repeat(64)),
      generalSemInteractionV2Intent(focalRelationId),
    )).toThrowError(expect.objectContaining({ code: "standard_sem_authority.interaction_duplicate" }));

    const withControl = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_relationship",
      relationship_id: "control:z-y",
      definition: { kind: "control", source: "construct:z", target: "construct:y", label: "Z control" },
    });
    expect(() => reduceStandardSemModelV4AuthorityV1(
      authority(withControl.model, "1".repeat(64)),
      generalSemInteractionV2Intent(focalRelationId),
    )).toThrowError(expect.objectContaining({
      code: "standard_sem_authority.general_sem_interaction_v2_main_effect_conflicts_control",
    }));
    expect(withControl.model.derived_terms).toHaveLength(0);
  });

  it.each([
    ["legacy interaction", interactionAuthority],
    ["interaction_v2", interactionV2Authority],
  ])("rejects a semantic duplicate of an existing %s even when IDs differ", (_label, sourceFactory) => {
    const source = sourceFactory();
    const existing = source.model.derived_terms.find((term) => term.kind === "interaction" || term.kind === "interaction_v2");
    if (!existing) throw new Error("Expected an existing interaction term.");

    expect(() => reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_interaction",
      term_id: "interaction:duplicate-with-fresh-id",
      output_id: "derived:duplicate-with-fresh-id",
      label: "Duplicate X by Z",
      predictor: "construct:x",
      moderator: "construct:z",
      focal_relation: existing.focal_relation,
      outcome: "construct:y",
      method: "two_stage",
    })).toThrowError(expect.objectContaining({
      code: "standard_sem_authority.interaction_duplicate",
      subject: "interaction:duplicate-with-fresh-id",
      corrective_action: expect.stringContaining("different moderator or focal relationship"),
    }));
  });

  it("accepts distinct moderators on one focal path and the same moderator on a distinct focal path", () => {
    const source = interactionAuthority();
    const focal = source.model.relations.find((relation) => relation.kind === "structural"
      && relation.source === "construct:x"
      && relation.target === "construct:y");
    if (!focal) throw new Error("Expected the original focal relation.");

    const withSecondModerator = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_construct",
      variable_id: "construct:w",
      label: "Moderator W",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [observed("observed:w1")],
    });
    const distinctModerator = reduceStandardSemModelV4AuthorityV1(authority(withSecondModerator.model, "d".repeat(64)), {
      kind: "add_interaction",
      term_id: "interaction:x-w",
      output_id: "derived:x-w",
      label: "X by W",
      predictor: "construct:x",
      moderator: "construct:w",
      focal_relation: focal.id,
      outcome: "construct:y",
      method: "two_stage",
    });
    expect(distinctModerator.model.derived_terms.filter((term) => term.kind === "interaction")).toHaveLength(2);

    const withSecondOutcome = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "add_construct",
      variable_id: "construct:q",
      label: "Outcome Q",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [observed("observed:q1")],
    });
    const withSecondFocal = reduceStandardSemModelV4AuthorityV1(authority(withSecondOutcome.model, "e".repeat(64)), {
      kind: "add_relationship",
      relationship_id: "relation:x-q",
      definition: { kind: "structural", source: "construct:x", target: "construct:q", label: "X to Q" },
    });
    const distinctFocalPath = reduceStandardSemModelV4AuthorityV1(authority(withSecondFocal.model, "f".repeat(64)), {
      kind: "add_interaction",
      term_id: "interaction:x-z-q",
      output_id: "derived:x-z-q",
      label: "X by Z on Q",
      predictor: "construct:x",
      moderator: "construct:z",
      focal_relation: "relation:x-q",
      outcome: "construct:q",
      method: "two_stage",
    });
    expect(distinctFocalPath.model.derived_terms.filter((term) => term.kind === "interaction")).toHaveLength(2);
  });

  it("cascades interaction_v2 references when an operand or focal relation is deleted", () => {
    const source = interactionV2Authority();
    const term = source.model.derived_terms.find((candidate) => candidate.kind === "interaction_v2");
    if (term?.kind !== "interaction_v2") throw new Error("Expected an interaction_v2 term.");

    const withoutModerator = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "delete_construct",
      variable_id: "construct:z",
    });
    expect(withoutModerator.model.variables.some((variable) => variable.id === term.output)).toBe(false);
    expect(withoutModerator.model.derived_terms.some((candidate) => candidate.id === term.id)).toBe(false);

    const withoutFocal = reduceStandardSemModelV4AuthorityV1(source, {
      kind: "delete_relationship",
      relationship_id: term.focal_relation,
    });
    expect(withoutFocal.model.variables.some((variable) => variable.id === term.output)).toBe(false);
    expect(withoutFocal.model.derived_terms.some((candidate) => candidate.id === term.id)).toBe(false);
  });

  it("authors and removes a strong-hierarchy three-way moderation without persisting a path target", () => {
    const base = generalSemModerationBase();
    const withSecondModerator = reduceStandardSemModelV4AuthorityV1(base.source, {
      kind: "add_construct",
      variable_id: "construct:w",
      label: "Second moderator",
      representation: { kind: "composite", weighting: { kind: "mode_a" } },
      indicators: [observed("observed:w1")],
    });
    const twoWay = reduceStandardSemModelV4AuthorityV1(authority(withSecondModerator.model, "e".repeat(64)), {
      kind: "add_moderating_effect_v3",
      intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
      sem_generation: "general_sem_v1",
      label: "X × Z",
      operands: ["construct:x", "construct:z"],
      target: { kind: "focal_relation", relationId: base.focalRelationId },
      outcome: "construct:y",
      method: "two_stage",
      hierarchy_policy: "strong",
    });
    const parent = twoWay.model.derived_terms.find((term) => term.kind === "interaction_v2");
    if (parent?.kind !== "interaction_v2") throw new Error("Expected the parent interaction.");

    const threeWay = reduceStandardSemModelV4AuthorityV1(authority(twoWay.model, "f".repeat(64)), {
      kind: "add_moderating_effect_v3",
      intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
      sem_generation: "general_sem_v1",
      label: "X × Z × W",
      operands: ["construct:x", "construct:z", "construct:w"],
      target: { kind: "parent_interaction", interactionTermId: parent.id },
      outcome: "construct:y",
      method: "two_stage",
      hierarchy_policy: "strong",
    });
    const topId = standardSemGeneralSemModerationV3ThreeWayTermIdV1(parent.id, "construct:w");
    const top = threeWay.model.derived_terms.find((term) => term.id === topId);
    expect(top).toEqual(expect.objectContaining({
      kind: "interaction_v2",
      operands: ["construct:x", "construct:z", "construct:w"],
      focal_relation: base.focalRelationId,
      hierarchy_policy: "strong",
    }));
    expect(threeWay.model.derived_terms.filter((term) => term.kind === "interaction_v2")).toHaveLength(4);
    expect(topId).toMatch(/^general_sem_v1_moderation_term_[0-9a-f_]+$/);
    expect(threeWay.model.annotations.every((annotation) => !annotation.id.includes("%"))).toBe(true);
    expect(threeWay.model.relations.every((relation) => !Object.hasOwn(relation, "target_relation"))).toBe(true);

    expect(() => reduceStandardSemModelV4AuthorityV1(authority(threeWay.model, "9".repeat(64)), {
      kind: "add_moderating_effect_v3",
      intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
      sem_generation: "general_sem_v1",
      label: "Second three-way effect",
      operands: ["construct:x", "construct:z", "construct:w"],
      target: { kind: "parent_interaction", interactionTermId: parent.id },
      outcome: "construct:y",
      method: "two_stage",
      hierarchy_policy: "strong",
    })).toThrow(/one three-way moderating effect per model/i);

    expect(() => reduceStandardSemModelV4AuthorityV1(authority(threeWay.model, "0".repeat(64)), {
      kind: "remove_moderating_effect",
      intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
      sem_generation: "general_sem_v1",
      term_id: parent.id,
      output_id: parent.output,
    })).toThrow(/required by a three-way effect/i);

    if (top?.kind !== "interaction_v2") throw new Error("Expected the three-way interaction.");
    const removed = reduceStandardSemModelV4AuthorityV1(authority(threeWay.model, "1".repeat(64)), {
      kind: "remove_moderating_effect",
      intent_version: GENERAL_SEM_MODERATING_EFFECT_INTENT_VERSION_V3,
      sem_generation: "general_sem_v1",
      term_id: top.id,
      output_id: top.output,
    });
    expect(removed.model.derived_terms).toContainEqual(expect.objectContaining({ id: parent.id }));
    expect(removed.model.derived_terms.some((term) => term.id === top.id)).toBe(false);
    expect(removed.model.derived_terms.filter((term) => term.kind === "interaction_v2")).toHaveLength(1);
  });
});
