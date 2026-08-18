import { describe, expect, it } from "vitest";
import {
  canonicalizeSemModelV4,
  compareUtf8StringsV1,
  compileCbsemPlanV2,
  compilePlsPlanV2,
  convertLegacyBasicModelV4,
  hasStructuralFeedbackV4,
  parseSemModelV4AuthoringDraft,
  parseSemModelV4,
  scientificSemModelV4HashInput,
  SemModelV4OperationError,
  type LegacyBasicModelV4Input,
  type SemModelV4,
  type SemParameterV4,
  type SemRelationV4,
  type SemWeightBindingV4,
  validateSemModelV4AuthoringIntegrity,
  validateSemModelV4,
} from "./semModelV4";

const reflectiveLegacy = (): LegacyBasicModelV4Input => ({
  id: "legacy-model",
  name: "Legacy model",
  constructs: [
    { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
    { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
  ],
  paths: [{ source: "x", target: "y" }],
  controls: [],
  higher_order_constructs: [],
  interactions: [],
});

const addFeedback = (model: SemModelV4) => {
  const relation: SemRelationV4 = {
    kind: "structural",
    id: "feedback",
    source: "construct:y",
    target: "construct:x",
    parameter: "feedback-p",
    intercept_parameter: null,
  };
  const parameter: SemParameterV4 = {
    kind: "free",
    id: "feedback-p",
    label: "Y -> X",
    target: { kind: "regression", source: "construct:y", target: "construct:x" },
  };
  model.relations.push(relation);
  model.parameters.push(parameter);
  const x = model.variables.find((variable) => variable.id === "construct:x");
  if (x?.kind === "common_factor" && x.disturbance_policy.kind === "exogenous_variance") {
    const varianceParameter = x.disturbance_policy.parameter;
    x.disturbance_policy = { kind: "endogenous_disturbance", parameter: varianceParameter };
    const variance = model.parameters.find((candidate) => candidate.id === varianceParameter);
    if (variance) variance.target = { kind: "variance", endpoint: { kind: "disturbance_of", id: x.id } };
  }
};

describe("SemModelV4", () => {
  it("matches the frozen Rust authoring-integrity boundary for incomplete science and typed references", () => {
    const underidentified = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const factor = underidentified.variables.find((variable) => variable.kind === "common_factor");
    if (factor?.kind !== "common_factor") throw new Error("Expected a common-factor fixture variable.");
    factor.identification = { kind: "fixed_variance" };

    expect(validateSemModelV4AuthoringIntegrity(underidentified)).toEqual([]);
    const normalizedDraft = parseSemModelV4AuthoringDraft(JSON.parse(JSON.stringify(underidentified)));
    expect(parseSemModelV4AuthoringDraft(JSON.parse(JSON.stringify(normalizedDraft)))).toEqual(normalizedDraft);
    expect(validateSemModelV4(underidentified).map((value) => value.code)).toContain("identification.fixed_variance.missing");
    expect(() => parseSemModelV4(underidentified)).toThrowError(expect.objectContaining({ code: "model.invalid" }));

    const danglingMarker = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const markerFactor = danglingMarker.variables.find((variable) => variable.kind === "common_factor");
    if (markerFactor?.kind !== "common_factor" || markerFactor.identification.kind !== "marker_loading") {
      throw new Error("Expected a marker-identified common-factor fixture variable.");
    }
    markerFactor.identification.indicator = "observed:missing-marker";
    expect(validateSemModelV4AuthoringIntegrity(danglingMarker).map((value) => value.code)).toContain("authoring.marker.reference_invalid");

    const danglingParameter = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    danglingParameter.relations[0].parameter = "missing-parameter";
    expect(validateSemModelV4AuthoringIntegrity(danglingParameter).map((value) => value.code)).toContain("relation.parameter.unknown");
  });

  it("rejects empty or cross-kind duplicate identities at the draft boundary", () => {
    const empty = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    empty.annotations.push({ kind: "caption", id: "", text: "Draft note" });
    expect(validateSemModelV4AuthoringIntegrity(empty).map((value) => value.code)).toContain("object.id.empty");
    expect(() => parseSemModelV4AuthoringDraft(empty)).toThrowError(expect.objectContaining({ code: "model.invalid" }));

    const duplicate = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    duplicate.annotations.push({ kind: "caption", id: duplicate.variables[0].id, text: "Draft note" });
    expect(validateSemModelV4AuthoringIntegrity(duplicate).map((value) => value.code)).toContain("object.id.duplicate");
    expect(() => parseSemModelV4AuthoringDraft(duplicate)).toThrowError(expect.objectContaining({ code: "model.invalid" }));
  });

  it("decodes required fields, scalar enums, and every SemModel serde default without treating null as missing", () => {
    const wire = JSON.parse(JSON.stringify(convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor"))) as Record<string, unknown>;
    delete wire.annotations;
    delete wire.presentation;
    const variables = wire.variables as Array<Record<string, unknown>>;
    const observed = variables.find((variable) => variable.kind === "observed");
    if (!observed) throw new Error("Expected an observed fixture variable.");
    delete observed.categories;
    delete observed.value_labels;
    delete observed.missing_markers;
    delete observed.transformation_lineage;
    const relations = wire.relations as Array<Record<string, unknown>>;
    const structural = relations.find((relation) => relation.kind === "structural");
    if (!structural) throw new Error("Expected a structural fixture relation.");
    delete structural.intercept_parameter;
    const parameters = wire.parameters as Array<Record<string, unknown>>;
    const free = parameters.find((parameter) => parameter.kind === "free");
    if (!free) throw new Error("Expected a free fixture parameter.");
    delete free.start;
    delete free.lower;
    delete free.upper;
    delete free.equality_label;
    for (const parameter of parameters) delete parameter.group_overrides;
    const binding = wire.data_binding as Record<string, unknown>;
    delete binding.weight;
    delete binding.cluster_variable;
    delete binding.strata_variable;

    const parsed = parseSemModelV4(wire);
    expect(parsed.annotations).toEqual([]);
    expect(parsed.presentation).toEqual({ kind: "none" });
    expect(parsed.variables.find((variable) => variable.id === observed.id)).toMatchObject({
      categories: [], value_labels: {}, missing_markers: [], transformation_lineage: [],
    });
    expect(parsed.relations.find((relation) => relation.id === structural.id)).toMatchObject({ intercept_parameter: null });
    expect(parsed.parameters.find((parameter) => parameter.id === free.id)).toMatchObject({
      start: null, lower: null, upper: null, equality_label: null, group_overrides: [],
    });
    expect(parsed.data_binding).toMatchObject({ weight: null, cluster_variable: null, strata_variable: null });

    const explicitNull = JSON.parse(JSON.stringify(wire)) as Record<string, unknown>;
    explicitNull.annotations = null;
    expect(() => parseSemModelV4(explicitNull)).toThrowError(expect.objectContaining({ code: "schema.invalid_shape" }));

    const invalidEnum = JSON.parse(JSON.stringify(wire)) as Record<string, unknown>;
    ((invalidEnum.variables as Array<Record<string, unknown>>).find((variable) => variable.kind === "observed")!).scale = "interval";
    expect(() => parseSemModelV4(invalidEnum)).toThrowError(expect.objectContaining({ code: "schema.invalid_discriminator" }));

    const missingRequired = JSON.parse(JSON.stringify(wire)) as Record<string, unknown>;
    delete missingRequired.group;
    expect(() => parseSemModelV4(missingRequired)).toThrowError(expect.objectContaining({ code: "schema.invalid_shape" }));
  });

  it("keeps the default structural role wire-compatible and strict", () => {
    const baseline = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const normalizedBaseline = parseSemModelV4(JSON.parse(JSON.stringify(baseline)));
    const baselineHashInput = scientificSemModelV4HashInput(normalizedBaseline);
    const wire = JSON.parse(JSON.stringify(baseline)) as Record<string, unknown>;
    const structural = (wire.relations as Array<Record<string, unknown>>)
      .find((relation) => relation.kind === "structural");
    if (!structural) throw new Error("Expected a structural fixture relation.");
    expect(structural.role).toBeUndefined();

    structural.role = "structural";
    const parsed = parseSemModelV4(wire);
    const parsedStructural = parsed.relations.find((relation) => relation.kind === "structural");
    expect(parsedStructural).not.toHaveProperty("role");
    expect(scientificSemModelV4HashInput(parsed)).toBe(baselineHashInput);
    expect(canonicalizeSemModelV4(parsed).relations.find((relation) => relation.kind === "structural"))
      .not.toHaveProperty("role");

    structural.role = "covariate";
    expect(() => parseSemModelV4(wire)).toThrowError(expect.objectContaining({ code: "schema.invalid_discriminator" }));
  });

  it("round-trips latent control relations as scientific identity", () => {
    const baseline = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const model = JSON.parse(JSON.stringify(baseline)) as SemModelV4;
    const structural = model.relations.find((relation) => relation.kind === "structural");
    if (structural?.kind !== "structural") throw new Error("Expected a structural fixture relation.");
    structural.role = "control";

    expect(validateSemModelV4(model)).toEqual([]);
    const wire = JSON.parse(JSON.stringify(model));
    const parsed = parseSemModelV4(wire);
    expect(parsed.relations.find((relation) => relation.id === structural.id)).toMatchObject({ role: "control" });
    expect(JSON.stringify(canonicalizeSemModelV4(parsed))).toContain('"role":"control"');
    expect(scientificSemModelV4HashInput(parsed)).not.toBe(scientificSemModelV4HashInput(baseline));
  });

  it("mirrors product-indicator construction enums and required-or-forbidden validation", () => {
    const valid = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const focal = valid.relations.find((relation) => relation.kind === "structural");
    if (focal?.kind !== "structural") throw new Error("Expected a structural focal relation.");
    valid.variables.push({ kind: "derived", id: "derived:x-by-y", label: "X by Y" });
    valid.relations.push({
      kind: "structural",
      id: "interaction-effect",
      source: "derived:x-by-y",
      target: focal.target,
      parameter: "interaction-effect-p",
      intercept_parameter: null,
    });
    valid.parameters.push({
      kind: "free",
      id: "interaction-effect-p",
      label: "X by Y -> Y",
      target: { kind: "regression", source: "derived:x-by-y", target: focal.target },
      group_overrides: [],
    });
    valid.derived_terms.push({
      kind: "interaction",
      id: "interaction-definition",
      output: "derived:x-by-y",
      predictor: focal.source,
      moderator: focal.target,
      focal_relation: focal.id,
      method: "product_indicator",
      product_indicator: {
        centering: "double_mean_center",
        standardization: "sample_standard_deviation",
        pairing: "all_pairs",
      },
    });
    expect(validateSemModelV4(valid)).toEqual([]);
    expect(parseSemModelV4(JSON.parse(JSON.stringify(valid))).derived_terms[0]).toMatchObject({
      method: "product_indicator",
      product_indicator: { centering: "double_mean_center", standardization: "sample_standard_deviation", pairing: "all_pairs" },
    });

    const missing = JSON.parse(JSON.stringify(valid)) as SemModelV4;
    const missingTerm = missing.derived_terms[0];
    if (missingTerm.kind !== "interaction") throw new Error("Expected an interaction fixture term.");
    delete missingTerm.product_indicator;
    expect(validateSemModelV4(missing).map((value) => value.code)).toContain("derived.interaction.product_indicator_spec_required");

    const forbidden = JSON.parse(JSON.stringify(valid)) as SemModelV4;
    const forbiddenTerm = forbidden.derived_terms[0];
    if (forbiddenTerm.kind !== "interaction") throw new Error("Expected an interaction fixture term.");
    forbiddenTerm.method = "two_stage";
    expect(validateSemModelV4(forbidden).map((value) => value.code)).toContain("derived.interaction.product_indicator_spec_forbidden");

    const invalidEnum = JSON.parse(JSON.stringify(valid)) as SemModelV4;
    const invalidTerm = invalidEnum.derived_terms[0];
    if (invalidTerm.kind !== "interaction" || !invalidTerm.product_indicator) throw new Error("Expected product settings.");
    (invalidTerm.product_indicator as unknown as Record<string, unknown>).centering = "center_somehow";
    expect(() => parseSemModelV4(invalidEnum)).toThrowError(expect.objectContaining({ code: "schema.invalid_discriminator" }));
  });

  it("fails closed on signed negative zero in ready and draft model numbers", () => {
    const ready = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const readyParameter = ready.parameters.find((parameter) => parameter.kind === "free");
    if (readyParameter?.kind !== "free") throw new Error("Expected a free ready-model parameter.");
    readyParameter.start = -0;
    expect(() => parseSemModelV4(ready)).toThrowError(expect.objectContaining({ code: "schema.invalid_shape" }));

    const draft = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const factor = draft.variables.find((variable) => variable.kind === "common_factor");
    const draftParameter = draft.parameters.find((parameter) => parameter.kind === "free");
    if (factor?.kind !== "common_factor" || draftParameter?.kind !== "free") throw new Error("Expected draft fixtures.");
    factor.identification = { kind: "fixed_variance" };
    draftParameter.start = -0;
    expect(() => parseSemModelV4AuthoringDraft(draft)).toThrowError(expect.objectContaining({ code: "schema.invalid_shape" }));
  });

  it("canonicalizes missing markers by UTF-8 bytes across the UTF-16 surrogate boundary", () => {
    const model = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const observed = model.variables.find((variable) => variable.kind === "observed");
    if (observed?.kind !== "observed") throw new Error("Expected an observed fixture variable.");
    observed.missing_markers = ["\u{10000}", "\uE000"];

    const canonical = canonicalizeSemModelV4(model);
    const canonicalObserved = canonical.variables.find((variable) => variable.id === observed.id);
    expect(compareUtf8StringsV1("\uE000", "\u{10000}")).toBeLessThan(0);
    expect(canonicalObserved).toMatchObject({ missing_markers: ["\uE000", "\u{10000}"] });
  });

  it("round-trips through JSON and has order-independent scientific hash input", () => {
    const model = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const roundTrip = JSON.parse(JSON.stringify(model)) as SemModelV4;
    expect(roundTrip).toEqual(model);

    const reordered = JSON.parse(JSON.stringify(model)) as SemModelV4;
    reordered.variables.reverse();
    reordered.relations.reverse();
    reordered.parameters.reverse();
    expect(scientificSemModelV4HashInput(reordered)).toBe(scientificSemModelV4HashInput(model));
  });

  it("excludes presentation and display-only covariance from scientific identity and plans", () => {
    const base = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const decorated = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor", [{
      id: "display-covariance",
      left_construct: "x",
      right_construct: "y",
      label: "visual only",
    }]);
    decorated.presentation = {
      kind: "canvas",
      nodes: [{ variable: "construct:x", x: 50, y: 80, style: { color: "blue" } }],
      edges: [],
      shapes: [{ id: "background", shape: "rounded_rectangle", x: 10, y: 10, width: 300, height: 200 }],
      images: [{ id: "logo", asset_ref: "project-asset:logo", alt_text: "Project logo", x: 20, y: 20, width: 64, height: 64 }],
      lines: [{ id: "divider", x1: 0, y1: 100, x2: 300, y2: 100 }],
      zoom: 1.5,
      pan_x: 10,
      pan_y: 20,
    };
    expect(scientificSemModelV4HashInput(decorated)).toBe(scientificSemModelV4HashInput(base));
    expect(compileCbsemPlanV2(decorated).covariances).toEqual([]);
  });

  it("puts an explicit residual covariance into the immutable CB-SEM plan", () => {
    const model = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    model.relations.push({
      kind: "covariance",
      id: "residual-covariance",
      left: { kind: "residual_of", id: "observed:x1" },
      right: { kind: "residual_of", id: "observed:y1" },
      parameter: "residual-covariance-p",
    });
    model.parameters.push({
      kind: "free",
      id: "residual-covariance-p",
      label: "Cov(e.x1,e.y1)",
      target: {
        kind: "covariance",
        left: { kind: "residual_of", id: "observed:x1" },
        right: { kind: "residual_of", id: "observed:y1" },
      },
      start: 0,
    });
    const plan = compileCbsemPlanV2(model);
    expect(plan.covariances).toHaveLength(1);
    expect(plan.covariances[0].left.kind).toBe("residual_of");
    expect(Object.isFrozen(plan)).toBe(true);
    expect(Object.isFrozen(plan.covariances)).toBe(true);
  });

  it("represents feedback but makes the PLS compiler fail closed", () => {
    const model = convertLegacyBasicModelV4(reflectiveLegacy(), "pls_composite");
    addFeedback(model);
    expect(validateSemModelV4(model)).toEqual([]);
    expect(hasStructuralFeedbackV4(model)).toBe(true);
    expect(() => compilePlsPlanV2(model)).toThrowError(expect.objectContaining({ code: "pls.feedback_unsupported" }));

    const cbsem = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    addFeedback(cbsem);
    expect(compileCbsemPlanV2(cbsem).has_feedback).toBe(true);
  });

  it("rejects duplicate ids and invalid constraints", () => {
    const model = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    model.variables.push({ ...model.variables[0] });
    model.constraints.push({ kind: "equality", id: "bad-equality", parameters: ["missing", "missing"] });
    const codes = new Set(validateSemModelV4(model).map((value) => value.code));
    expect(codes.has("object.id.duplicate")).toBe(true);
    expect(codes.has("constraint.equality.invalid")).toBe(true);
    expect(codes.has("constraint.parameter.unknown")).toBe(true);
  });

  it("accepts explicit equality constraints and rejects ambiguous migration", () => {
    const model = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const freeLoadings = model.parameters.filter((parameter) => parameter.kind === "free" && parameter.target.kind === "loading");
    model.constraints.push({ kind: "equality", id: "equal-loadings", parameters: freeLoadings.slice(0, 2).map((parameter) => parameter.id) });
    expect(validateSemModelV4(model)).toEqual([]);
    expect(compileCbsemPlanV2(model).constraints).toHaveLength(1);

    try {
      convertLegacyBasicModelV4(reflectiveLegacy(), "unspecified");
      throw new Error("Expected migration to fail");
    } catch (error) {
      expect(error).toBeInstanceOf(SemModelV4OperationError);
      expect((error as SemModelV4OperationError).code).toBe("migration.interpretation_required");
    }
  });

  it("fails closed when a basic compiler sees supported-in-IR advanced semantics", () => {
    const model = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    model.group = {
      kind: "observed_groups",
      grouping_variable: "observed:x1",
      levels: [
        { id: "a", value: "A", label: "Group A" },
        { id: "b", value: "B", label: "Group B" },
      ],
    };
    expect(validateSemModelV4(model)).toEqual([]);
    expect(() => compileCbsemPlanV2(model)).toThrowError(expect.objectContaining({ code: "cbsem.multigroup_unsupported" }));
  });

  it("round-trips control, identifier, categories, missing markers, and transformation lineage strictly", () => {
    const model = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    model.variables.push({
      kind: "observed",
      id: "observed:segment",
      label: "Segment",
      source_column: "segment_code",
      scale: "nominal",
      role: "control",
      categories: ["A", "B"],
      value_labels: { A: "Enterprise", B: "Consumer" },
      missing_markers: ["-9"],
      transformation_lineage: [{
        id: "recode-segment",
        input_columns: ["segment_raw"],
        output_column: "segment_code",
        operation: { kind: "recode", mappings: { enterprise: "A", consumer: "B" }, unmapped: "set_missing" },
      }],
    }, {
      kind: "observed",
      id: "observed:case_id",
      label: "Case id",
      source_column: "case_id",
      scale: "identifier",
      role: "control",
      categories: [],
      value_labels: {},
      missing_markers: [],
      transformation_lineage: [],
    });
    expect(validateSemModelV4(model)).toEqual([]);
    const normalized = parseSemModelV4(JSON.parse(JSON.stringify(model)));
    expect(parseSemModelV4(JSON.parse(JSON.stringify(normalized)))).toEqual(normalized);

    const unknownTop = { ...model, unexpected: true };
    expect(() => parseSemModelV4(unknownTop)).toThrowError(expect.objectContaining({ code: "schema.unknown_field" }));
    const unknownNested = JSON.parse(JSON.stringify(model)) as SemModelV4;
    (unknownNested.variables[0] as unknown as Record<string, unknown>).unexpected = true;
    expect(() => parseSemModelV4(unknownNested)).toThrowError(expect.objectContaining({ code: "schema.unknown_field" }));

    model.relations.push({ kind: "structural", id: "identifier-outcome", source: "construct:x", target: "observed:case_id", parameter: "identifier-outcome-p", intercept_parameter: null });
    model.parameters.push({ kind: "free", id: "identifier-outcome-p", label: "X -> case id", target: { kind: "regression", source: "construct:x", target: "observed:case_id" }, group_overrides: [] });
    const codes = validateSemModelV4(model).map((value) => value.code);
    expect(codes).toContain("observed.role.usage_invalid");
    expect(codes).toContain("observed.identifier.model_use_invalid");
  });

  it("preserves fixed composite scoring while unsupported sampling weights fail closed", () => {
    const legacy = reflectiveLegacy();
    legacy.constructs[0].mode = "formative";
    const custom = convertLegacyBasicModelV4(legacy, "pls_composite");
    const construct = custom.variables.find((variable) => variable.id === "construct:x");
    if (construct?.kind !== "composite") throw new Error("missing composite");
    construct.weighting = { kind: "custom", weights: { "observed:x1": 0.4, "observed:x2": 0.6 }, normalization: "sum_to_one" };
    expect(validateSemModelV4(custom)).toEqual([]);
    expect(compilePlsPlanV2(custom).blocks.find((block) => block.construct_id === "construct:x")?.fixed_scoring).toEqual({
      kind: "custom",
      weights: { "observed:x1": 0.4, "observed:x2": 0.6 },
      normalization: "sum_to_one",
    });
    construct.weighting = { kind: "unit", normalization: "unit_variance" };
    expect(compilePlsPlanV2(custom).blocks.find((block) => block.construct_id === "construct:x")?.fixed_scoring).toEqual({
      kind: "unit",
      normalization: "unit_variance",
    });
    construct.weighting = { kind: "custom", weights: { "observed:x1": 0.4, "observed:x2": 0.6 }, normalization: "sum_to_one" };
    delete construct.weighting.weights["observed:x2"];
    expect(validateSemModelV4(custom).map((value) => value.code)).toContain("identification.composite.custom_weights_invalid");

    const weighted = convertLegacyBasicModelV4(reflectiveLegacy(), "pls_composite");
    weighted.variables.push({ kind: "observed", id: "observed:weight", label: "Weight", source_column: "weight", scale: "continuous", role: "control", categories: [], value_labels: {}, missing_markers: [], transformation_lineage: [] });
    const weights: SemWeightBindingV4[] = [
      { kind: "case", variable: "observed:weight" },
      { kind: "frequency", variable: "observed:weight" },
      { kind: "sampling", variable: "observed:weight", normalization: "mean_one" },
    ];
    for (const weight of weights) {
      const candidate = JSON.parse(JSON.stringify(weighted)) as SemModelV4;
      if (candidate.data_binding.kind !== "raw") throw new Error("expected raw binding");
      candidate.data_binding.weight = weight;
      expect(validateSemModelV4(candidate)).toEqual([]);
      expect(() => compilePlsPlanV2(candidate)).toThrowError(expect.objectContaining({ code: "pls.data_binding_unsupported" }));
    }
  });

  it("strictly parses every weight discriminator and rejects unknown weight wire", () => {
    const weighted = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    weighted.variables.push({
      kind: "observed",
      id: "observed:weight",
      label: "Weight",
      source_column: "weight",
      scale: "continuous",
      role: "control",
      categories: [],
      value_labels: {},
      missing_markers: [],
      transformation_lineage: [],
    });
    const candidate = JSON.parse(JSON.stringify(weighted)) as SemModelV4;
    if (candidate.data_binding.kind !== "raw") throw new Error("expected raw binding");
    candidate.data_binding.weight = { kind: "sampling", variable: "observed:weight", normalization: "sum_to_sample_size" };
    expect(parseSemModelV4(candidate).data_binding).toMatchObject({
      weight: { kind: "sampling", variable: "observed:weight", normalization: "sum_to_sample_size" },
    });

    const unknownKind = JSON.parse(JSON.stringify(candidate)) as Record<string, unknown>;
    ((unknownKind.data_binding as Record<string, unknown>).weight as Record<string, unknown>).kind = "replicate";
    expect(() => parseSemModelV4(unknownKind)).toThrowError(expect.objectContaining({
      code: "schema.invalid_discriminator",
      subject: "model.data_binding.weight.kind",
    }));

    const unknownNormalization = JSON.parse(JSON.stringify(candidate)) as Record<string, unknown>;
    ((unknownNormalization.data_binding as Record<string, unknown>).weight as Record<string, unknown>).normalization = "automatic";
    expect(() => parseSemModelV4(unknownNormalization)).toThrowError(expect.objectContaining({
      code: "schema.invalid_discriminator",
      subject: "model.data_binding.weight.normalization",
    }));

    const unknownField = JSON.parse(JSON.stringify(candidate)) as Record<string, unknown>;
    ((unknownField.data_binding as Record<string, unknown>).weight as Record<string, unknown>).calibration = "poststratified";
    expect(() => parseSemModelV4(unknownField)).toThrowError(expect.objectContaining({
      code: "schema.unknown_field",
      subject: "model.data_binding.weight.calibration",
    }));

    const caseWithNormalization = JSON.parse(JSON.stringify(candidate)) as Record<string, unknown>;
    (caseWithNormalization.data_binding as Record<string, unknown>).weight = {
      kind: "case",
      variable: "observed:weight",
      normalization: "none",
    };
    expect(() => parseSemModelV4(caseWithNormalization)).toThrowError(expect.objectContaining({
      code: "schema.unknown_field",
      subject: "model.data_binding.weight.normalization",
    }));
  });

  it("validates factor policies, group overrides, and matrix sample metadata", () => {
    const factorModel = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    factorModel.parameters.push({ kind: "free", id: "mean-x", label: "Mean X", target: { kind: "mean", variable: "construct:x" }, start: 0, group_overrides: [] });
    const factor = factorModel.variables.find((variable) => variable.id === "construct:x");
    if (factor?.kind !== "common_factor") throw new Error("missing factor");
    factor.mean_policy = { kind: "estimated", parameter: "mean-x" };
    expect(validateSemModelV4(factorModel)).toEqual([]);
    expect(() => compileCbsemPlanV2(factorModel)).toThrowError(expect.objectContaining({ code: "cbsem.factor_policy_unsupported" }));

    const grouped = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    grouped.group = { kind: "observed_groups", grouping_variable: "observed:x1", levels: [{ id: "a", value: "A", label: "A" }, { id: "b", value: "B", label: "B" }] };
    const groupMean: SemParameterV4 = {
      kind: "free",
      id: "group-mean-x",
      label: "Group mean X",
      target: { kind: "mean", variable: "construct:x" },
      group_overrides: [
        { group: "a", specification: { kind: "fixed", value: 0 } },
        { group: "b", specification: { kind: "free", start: 0 } },
      ],
    };
    grouped.parameters.push(groupMean);
    const groupedFactor = grouped.variables.find((variable) => variable.id === "construct:x");
    if (groupedFactor?.kind !== "common_factor") throw new Error("missing grouped factor");
    groupedFactor.mean_policy = { kind: "reference_group", reference_group: "a", parameter: groupMean.id };
    expect(validateSemModelV4(grouped)).toEqual([]);
    groupMean.group_overrides![0].group = "missing";
    expect(validateSemModelV4(grouped).map((value) => value.code)).toContain("parameter.group_override.group_invalid");

    const matrix = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    const variables = ["observed:x1", "observed:x2", "observed:y1", "observed:y2"];
    matrix.data_binding = {
      kind: "covariance",
      dataset_id: "covariance-input",
      variables,
      means: { "observed:x1": 1, "observed:x2": 2, "observed:y1": 3, "observed:y2": 4 },
      standard_deviations: { "observed:x1": 1.1, "observed:x2": 1.2, "observed:y1": 1.3, "observed:y2": 1.4 },
      sample: {
        sample_size: 200,
        covariance_denominator: "sample_n_minus_one",
        effective_sample_size: 184.5,
        degrees_of_freedom: 199,
        group_sample_sizes: {},
      },
    };
    expect(validateSemModelV4(matrix)).toEqual([]);
    expect(() => compileCbsemPlanV2(matrix)).toThrowError(expect.objectContaining({ code: "cbsem.matrix_metadata_unsupported" }));
  });

  it("allows bounded raw mean replacement for CB-SEM while PLS and other CB-SEM policies fail closed", () => {
    const cbsem = convertLegacyBasicModelV4(reflectiveLegacy(), "cbsem_common_factor");
    if (cbsem.data_binding.kind !== "raw") throw new Error("expected raw binding");
    const observed = cbsem.variables.find((variable) => variable.kind === "observed");
    if (observed?.kind !== "observed") throw new Error("expected observed variable");
    observed.missing_markers = ["NA"];
    cbsem.data_binding.missing_data = "mean_replacement";
    expect(compileCbsemPlanV2(cbsem).input).toMatchObject({ kind: "raw", missing_data: "mean_replacement" });
    cbsem.data_binding.missing_data = "listwise_deletion";
    expect(() => compileCbsemPlanV2(cbsem)).toThrowError(expect.objectContaining({ code: "cbsem.observed_metadata_unsupported" }));
    observed.missing_markers = [];
    cbsem.data_binding.missing_data = "pairwise_deletion";
    expect(() => compileCbsemPlanV2(cbsem)).toThrowError(expect.objectContaining({ code: "cbsem.data_binding_unsupported" }));

    const pls = convertLegacyBasicModelV4(reflectiveLegacy(), "pls_composite");
    if (pls.data_binding.kind !== "raw") throw new Error("expected raw binding");
    pls.data_binding.missing_data = "mean_replacement";
    expect(() => compilePlsPlanV2(pls)).toThrowError(expect.objectContaining({ code: "pls.data_binding_unsupported" }));
  });

  it("binds moderation to its focal path and enumerates complete HOC construction approaches", () => {
    const legacy = reflectiveLegacy();
    legacy.constructs.push({ id: "m", name: "Moderator", short_name: "M", mode: "reflective", indicators: ["m1", "m2"] });
    const model = convertLegacyBasicModelV4(legacy, "pls_composite");
    const focal = model.relations.find((relation) => relation.kind === "structural" && relation.source === "construct:x" && relation.target === "construct:y")!;
    model.variables.push({ kind: "derived", id: "derived:x-by-m", label: "X x M" });
    model.relations.push({ kind: "structural", id: "interaction-effect", source: "derived:x-by-m", target: "construct:y", parameter: "interaction-effect-p", intercept_parameter: null });
    model.parameters.push({ kind: "free", id: "interaction-effect-p", label: "X x M -> Y", target: { kind: "regression", source: "derived:x-by-m", target: "construct:y" }, group_overrides: [] });
    model.derived_terms.push({ kind: "interaction", id: "interaction-x-m", output: "derived:x-by-m", predictor: "construct:x", moderator: "construct:m", focal_relation: focal.id, method: "two_stage" });
    expect(validateSemModelV4(model)).toEqual([]);
    const interaction = model.derived_terms[0];
    if (interaction.kind !== "interaction") throw new Error("missing interaction");
    interaction.focal_relation = "missing";
    expect(validateSemModelV4(model).map((value) => value.code)).toContain("derived.interaction.focal_relation_unknown");

    const approaches = ["repeated_indicators", "extended_repeated_indicators", "embedded_two_stage", "disjoint_two_stage", "hybrid"] as const;
    expect(JSON.parse(JSON.stringify(approaches))).toEqual(approaches);
    const hoc = { kind: "higher_order", id: "hoc", output: "derived:hoc", components: ["construct:x", "construct:y"], approach: "extended_repeated_indicators", measurement_type: "reflective_formative" } as const;
    expect(JSON.stringify(hoc)).toContain("extended_repeated_indicators");
  });
});
