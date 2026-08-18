import { describe, expect, it } from "vitest";
import {
  canonicalizeSemModelV4,
  convertLegacyBasicModelV4,
  parseSemModelV4,
  scientificSemModelV4HashInput,
  type InteractionHierarchyPolicyV2,
  type InteractionMethodV4,
  type LegacyBasicModelV4Input,
  type ProductIndicatorSpecificationV4,
  type SemDerivedTermV4,
  type SemModelV4,
  type SemRelationV4,
  validateSemModelV4,
  validateSemModelV4AuthoringIntegrity,
} from "./semModelV4";

const CONSTRUCT = {
  x: "construct:x",
  m: "construct:m",
  w: "construct:w",
  z: "construct:z",
  y: "construct:y",
} as const;

type InputId = Exclude<keyof typeof CONSTRUCT, "y">;

function interactionFixture(mainEffects: readonly InputId[] = ["x", "m", "w", "z"]): SemModelV4 {
  const constructs: LegacyBasicModelV4Input["constructs"] = [
    ["x", "Predictor X"],
    ["m", "Moderator M"],
    ["w", "Moderator W"],
    ["z", "Moderator Z"],
    ["y", "Outcome Y"],
  ].map(([id, name]) => ({
    id,
    name,
    short_name: id.toUpperCase(),
    mode: "reflective" as const,
    indicators: [`${id}1`, `${id}2`],
  }));
  return convertLegacyBasicModelV4({
    id: "interaction-v2-model",
    name: "Interaction V2 model",
    constructs,
    paths: mainEffects.map((source) => ({ source, target: "y" })),
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  }, "pls_composite");
}

function structuralEffect(
  model: SemModelV4,
  source: string,
  target = CONSTRUCT.y,
): Extract<SemRelationV4, { kind: "structural" }> {
  const relation = model.relations.find((candidate): candidate is Extract<SemRelationV4, { kind: "structural" }> =>
    candidate.kind === "structural"
    && candidate.role !== "control"
    && candidate.source === source
    && candidate.target === target);
  if (!relation) throw new Error(`Expected structural-effect path ${source} -> ${target}.`);
  return relation;
}

function addStructuralEffect(model: SemModelV4, relationId: string, source: string, target = CONSTRUCT.y) {
  const parameterId = `parameter:${relationId}`;
  model.relations.push({
    kind: "structural",
    id: relationId,
    source,
    target,
    parameter: parameterId,
    intercept_parameter: null,
  });
  model.parameters.push({
    kind: "free",
    id: parameterId,
    label: `${source} -> ${target}`,
    target: { kind: "regression", source, target },
    group_overrides: [],
  });
}

function addInteractionV2(
  model: SemModelV4,
  id: string,
  operands: readonly string[],
  hierarchyPolicy: InteractionHierarchyPolicyV2 = "strong",
  method: InteractionMethodV4 = "two_stage",
  productIndicator?: ProductIndicatorSpecificationV4,
): Extract<SemDerivedTermV4, { kind: "interaction_v2" }> {
  const output = `derived:${id}`;
  model.variables.push({ kind: "derived", id: output, label: operands.join(" x ") });
  addStructuralEffect(model, `relation:${id}:effect`, output);
  const term: Extract<SemDerivedTermV4, { kind: "interaction_v2" }> = {
    kind: "interaction_v2",
    id,
    output,
    operands: [...operands],
    focal_relation: structuralEffect(model, operands[0]!).id,
    method,
    hierarchy_policy: hierarchyPolicy,
    ...(productIndicator ? { product_indicator: productIndicator } : {}),
  };
  model.derived_terms.push(term);
  return term;
}

function removeInteraction(model: SemModelV4, termId: string) {
  const term = model.derived_terms.find((candidate) => candidate.id === termId);
  if (!term) throw new Error(`Expected interaction ${termId}.`);
  const effectRelations = model.relations.filter((relation) =>
    relation.kind === "structural" && relation.source === term.output);
  const effectRelationIds = new Set(effectRelations.map((relation) => relation.id));
  const effectParameterIds = new Set(effectRelations.map((relation) => relation.parameter));
  model.derived_terms = model.derived_terms.filter((candidate) => candidate.id !== termId);
  model.variables = model.variables.filter((variable) => variable.id !== term.output);
  model.relations = model.relations.filter((relation) => !effectRelationIds.has(relation.id));
  model.parameters = model.parameters.filter((parameter) => !effectParameterIds.has(parameter.id));
}

function issueCodes(model: SemModelV4) {
  return validateSemModelV4AuthoringIntegrity(model).map((value) => value.code);
}

describe("SemModelV4 interaction_v2", () => {
  it("keeps the existing two-way interaction wire shape readable and unchanged", () => {
    const model = interactionFixture(["x", "m"]);
    const output = "derived:legacy-x-m";
    model.variables.push({ kind: "derived", id: output, label: "Legacy X x M" });
    addStructuralEffect(model, "relation:legacy-x-m:effect", output);
    const legacy: Extract<SemDerivedTermV4, { kind: "interaction" }> = {
      kind: "interaction",
      id: "term:legacy-x-m",
      output,
      predictor: CONSTRUCT.x,
      moderator: CONSTRUCT.m,
      focal_relation: structuralEffect(model, CONSTRUCT.x).id,
      method: "two_stage",
    };
    model.derived_terms.push(legacy);

    expect(validateSemModelV4(model)).toEqual([]);
    const parsed = parseSemModelV4(JSON.parse(JSON.stringify(model)));
    expect(parsed.derived_terms).toEqual([legacy]);
    expect(parsed.derived_terms[0]).not.toHaveProperty("operands");
    expect(parsed.derived_terms[0]).not.toHaveProperty("hierarchy_policy");
  });

  it("strictly round-trips V2 while preserving operand order and scientific identity", () => {
    const model = interactionFixture(["x", "m"]);
    const term = addInteractionV2(model, "term:x:m", [CONSTRUCT.x, CONSTRUCT.m]);

    expect(validateSemModelV4(model)).toEqual([]);
    const parsed = parseSemModelV4(JSON.parse(JSON.stringify(model)));
    const parsedTerm = parsed.derived_terms.find((candidate) => candidate.id === term.id);
    expect(parsedTerm).toMatchObject({
      kind: "interaction_v2",
      operands: [CONSTRUCT.x, CONSTRUCT.m],
      hierarchy_policy: "strong",
    });
    expect(canonicalizeSemModelV4(parsed).derived_terms[0]).toMatchObject({
      operands: [CONSTRUCT.x, CONSTRUCT.m],
    });
    const reparsed = parseSemModelV4(JSON.parse(JSON.stringify(parsed)));
    expect(scientificSemModelV4HashInput(reparsed)).toBe(scientificSemModelV4HashInput(parsed));

    const explicitNull = JSON.parse(JSON.stringify(model)) as SemModelV4;
    (explicitNull.derived_terms[0] as Extract<SemDerivedTermV4, { kind: "interaction_v2" }>).product_indicator = null;
    expect(parseSemModelV4(explicitNull).derived_terms[0]).not.toHaveProperty("product_indicator");

    const unknownField = JSON.parse(JSON.stringify(model)) as SemModelV4;
    (unknownField.derived_terms[0] as unknown as Record<string, unknown>).moderator = CONSTRUCT.m;
    expect(() => parseSemModelV4(unknownField)).toThrowError(expect.objectContaining({ code: "schema.unknown_field" }));

    const invalidPolicy = JSON.parse(JSON.stringify(model)) as SemModelV4;
    (invalidPolicy.derived_terms[0] as unknown as Record<string, unknown>).hierarchy_policy = "automatic";
    expect(() => parseSemModelV4(invalidPolicy)).toThrowError(expect.objectContaining({ code: "schema.invalid_discriminator" }));
  });

  it("rejects insufficient, duplicate, output, and unknown operands at authoring integrity", () => {
    const baseline = interactionFixture(["x", "m"]);
    const term = addInteractionV2(baseline, "term:x:m", [CONSTRUCT.x, CONSTRUCT.m]);

    for (const operands of [
      [CONSTRUCT.x],
      [CONSTRUCT.x, CONSTRUCT.x],
      [CONSTRUCT.x, term.output],
    ]) {
      const invalid = structuredClone(baseline);
      (invalid.derived_terms[0] as Extract<SemDerivedTermV4, { kind: "interaction_v2" }>).operands = operands;
      expect(issueCodes(invalid)).toContain("derived.interaction_v2.operands_invalid");
    }

    const unknown = structuredClone(baseline);
    (unknown.derived_terms[0] as Extract<SemDerivedTermV4, { kind: "interaction_v2" }>).operands = [CONSTRUCT.x, "construct:missing"];
    expect(issueCodes(unknown)).toContain("derived.input.unknown");
  });

  it("enforces product specification coherence and structural-effect roles", () => {
    const missingSpecification = interactionFixture(["x", "m"]);
    addInteractionV2(missingSpecification, "term:x:m", [CONSTRUCT.x, CONSTRUCT.m], "strong", "product_indicator");
    expect(issueCodes(missingSpecification)).toContain("derived.interaction.product_indicator_spec_required");

    const productIndicator: ProductIndicatorSpecificationV4 = {
      centering: "double_mean_center",
      standardization: "sample_standard_deviation",
      pairing: "all_pairs",
    };
    const forbiddenSpecification = interactionFixture(["x", "m"]);
    addInteractionV2(forbiddenSpecification, "term:x:m", [CONSTRUCT.x, CONSTRUCT.m], "strong", "two_stage", productIndicator);
    expect(issueCodes(forbiddenSpecification)).toContain("derived.interaction.product_indicator_spec_forbidden");

    const controlFocal = interactionFixture(["x", "m"]);
    const controlFocalTerm = addInteractionV2(controlFocal, "term:x:m", [CONSTRUCT.x, CONSTRUCT.m]);
    const focal = controlFocal.relations.find((relation) => relation.id === controlFocalTerm.focal_relation);
    if (focal?.kind !== "structural") throw new Error("Expected focal relation.");
    focal.role = "control";
    expect(issueCodes(controlFocal)).toContain("derived.interaction.focal_relation_invalid");

    const controlEffect = interactionFixture(["x", "m"]);
    const controlEffectTerm = addInteractionV2(controlEffect, "term:x:m", [CONSTRUCT.x, CONSTRUCT.m]);
    const effect = controlEffect.relations.find((relation) => relation.kind === "structural" && relation.source === controlEffectTerm.output);
    if (effect?.kind !== "structural") throw new Error("Expected interaction effect relation.");
    effect.role = "control";
    expect(issueCodes(controlEffect)).toContain("derived.interaction.effect_path_missing");
  });

  it("requires main effects for weak and strong policies but not none", () => {
    const weak = interactionFixture(["x"]);
    addInteractionV2(weak, "term:x:m", [CONSTRUCT.x, CONSTRUCT.m], "weak");
    expect(issueCodes(weak)).toContain("derived.interaction_v2.main_effect_missing");

    const none = structuredClone(weak);
    (none.derived_terms[0] as Extract<SemDerivedTermV4, { kind: "interaction_v2" }>).hierarchy_policy = "none";
    expect(validateSemModelV4(none)).toEqual([]);
  });

  it("accepts a complete strong three-way hierarchy and detects its missing lower order", () => {
    const model = interactionFixture(["x", "m", "w"]);
    addInteractionV2(model, "term:x:m", [CONSTRUCT.x, CONSTRUCT.m]);
    addInteractionV2(model, "term:x:w", [CONSTRUCT.x, CONSTRUCT.w]);
    addInteractionV2(model, "term:m:w", [CONSTRUCT.m, CONSTRUCT.w]);
    const top = addInteractionV2(model, "term:x:m:w", [CONSTRUCT.x, CONSTRUCT.m, CONSTRUCT.w]);

    expect(validateSemModelV4(model)).toEqual([]);
    const parsed = parseSemModelV4(JSON.parse(JSON.stringify(model)));
    expect(parsed.derived_terms.find((term) => term.id === top.id)).toMatchObject({
      operands: [CONSTRUCT.x, CONSTRUCT.m, CONSTRUCT.w],
    });

    const reordered = structuredClone(model);
    const reorderedTop = reordered.derived_terms.find((term): term is Extract<SemDerivedTermV4, { kind: "interaction_v2" }> => term.id === top.id && term.kind === "interaction_v2");
    if (!reorderedTop) throw new Error("Expected top interaction.");
    [reorderedTop.operands[1], reorderedTop.operands[2]] = [reorderedTop.operands[2]!, reorderedTop.operands[1]!];
    expect(validateSemModelV4(reordered)).toEqual([]);
    expect(scientificSemModelV4HashInput(reordered)).not.toBe(scientificSemModelV4HashInput(model));

    const missingLowerOrder = structuredClone(model);
    removeInteraction(missingLowerOrder, "term:x:m");
    const missingIssues = validateSemModelV4AuthoringIntegrity(missingLowerOrder);
    expect(missingIssues).toContainEqual(expect.objectContaining({
      code: "derived.interaction_v2.lower_order_missing",
      subject: top.id,
    }));

    const weakTop = missingLowerOrder.derived_terms.find((term): term is Extract<SemDerivedTermV4, { kind: "interaction_v2" }> => term.id === top.id && term.kind === "interaction_v2");
    if (!weakTop) throw new Error("Expected top interaction.");
    weakTop.hierarchy_policy = "weak";
    expect(validateSemModelV4(missingLowerOrder)).toEqual([]);
  });

  it("requires higher-order lower terms to be transitively strong", () => {
    const model = interactionFixture(["x", "m", "w", "z"]);
    const operands = [CONSTRUCT.x, CONSTRUCT.m, CONSTRUCT.w, CONSTRUCT.z];
    for (let left = 0; left < operands.length; left += 1) {
      for (let right = left + 1; right < operands.length; right += 1) {
        addInteractionV2(model, `term:pair:${left}:${right}`, [operands[left]!, operands[right]!]);
      }
    }
    for (let omitted = 0; omitted < operands.length; omitted += 1) {
      addInteractionV2(model, `term:triple:${omitted}`, operands.filter((_, index) => index !== omitted));
    }
    const top = addInteractionV2(model, "term:four-way", operands);
    expect(validateSemModelV4(model)).toEqual([]);

    const weakLower = model.derived_terms.find((term): term is Extract<SemDerivedTermV4, { kind: "interaction_v2" }> => term.id === "term:triple:3" && term.kind === "interaction_v2");
    if (!weakLower) throw new Error("Expected a three-way lower-order interaction.");
    weakLower.hierarchy_policy = "weak";
    expect(validateSemModelV4AuthoringIntegrity(model)).toContainEqual(expect.objectContaining({
      code: "derived.interaction_v2.lower_order_missing",
      subject: top.id,
    }));
  });
});
