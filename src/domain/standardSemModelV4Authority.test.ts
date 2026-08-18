import { describe, expect, it } from "vitest";
import { convertLegacyBasicModelV4, parseSemModelV4AuthoringDraft, type LegacyBasicModelV4Input, type SemModelV4, type SemVariableV4 } from "./semModelV4";
import {
  parseStandardSemModelV4AuthorityRecordV1,
  reduceStandardSemModelV4AuthorityV1,
  StandardSemModelV4AuthorityError,
  standardSemMeasurementRelationIdV1,
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
});
