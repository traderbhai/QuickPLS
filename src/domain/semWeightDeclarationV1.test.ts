import { describe, expect, it } from "vitest";
import { convertLegacyBasicModelV4, type SemModelV4, type SemWeightBindingV4 } from "./semModelV4";
import {
  legacyCaseWeightCapabilityIssueV1,
  parseResolvedWeightDeclarationV1,
  parseWeightCapabilityIssueV1,
  resolveSemWeightDeclarationV1,
  samplingWeightNormalizationIssueV1,
  SemWeightDeclarationV1Error,
  weightCapabilityIssueV1,
  type ResolvedWeightDeclarationV1,
  type WeightCapabilityTargetV1,
} from "./semWeightDeclarationV1";

const targets: WeightCapabilityTargetV1[] = [
  "pls_plan_v2",
  "cbsem_ml_v1",
  "cbsem_ml_mean_replacement_v1",
  "cbsem_product_indicator_plan_v1",
];

function weightedModel(weight: SemWeightBindingV4): SemModelV4 {
  const model = convertLegacyBasicModelV4({
    id: "weighted-model",
    name: "Weighted model",
    constructs: [{ id: "x", name: "X", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] }],
    paths: [],
  }, "cbsem_common_factor");
  model.variables.push({
    kind: "observed",
    id: "observed:survey_weight",
    label: "Survey weight",
    source_column: "survey_weight",
    scale: "continuous",
    role: "control",
    categories: [],
    value_labels: {},
    missing_markers: [],
    transformation_lineage: [],
  });
  if (model.data_binding.kind !== "raw") throw new Error("expected a raw fixture binding");
  model.data_binding.dataset_id = "dataset-weighted";
  model.data_binding.weight = weight;
  return model;
}

describe("Sem weight declaration v1", () => {
  it("resolves case, frequency, and sampling bindings to stable variable/source identities", () => {
    expect(resolveSemWeightDeclarationV1(weightedModel({ kind: "case", variable: "observed:survey_weight" }))).toEqual({
      contract_version: "sem_weight_declaration_v1",
      dataset_id: "dataset-weighted",
      binding: { kind: "case", variable_id: "observed:survey_weight", source_column: "survey_weight" },
    });
    expect(resolveSemWeightDeclarationV1(weightedModel({ kind: "frequency", variable: "observed:survey_weight" }))).toMatchObject({
      binding: { kind: "frequency", variable_id: "observed:survey_weight", source_column: "survey_weight" },
    });
    expect(resolveSemWeightDeclarationV1(weightedModel({
      kind: "sampling",
      variable: "observed:survey_weight",
      normalization: "sum_to_sample_size",
    }))).toMatchObject({
      binding: {
        kind: "sampling",
        variable_id: "observed:survey_weight",
        source_column: "survey_weight",
        normalization: "sum_to_sample_size",
      },
    });
  });

  it("strictly parses resolved declarations and rejects kind-specific extra fields", () => {
    const declaration = resolveSemWeightDeclarationV1(weightedModel({
      kind: "sampling",
      variable: "observed:survey_weight",
      normalization: "mean_one",
    }))!;
    expect(parseResolvedWeightDeclarationV1(JSON.parse(JSON.stringify(declaration)))).toEqual(declaration);
    expect(() => parseResolvedWeightDeclarationV1({
      ...declaration,
      binding: { ...declaration.binding, normalization: "automatic" },
    })).toThrowError(expect.objectContaining({ code: "schema.invalid_discriminator" }));
    expect(() => parseResolvedWeightDeclarationV1({
      ...declaration,
      binding: { kind: "case", variable_id: "observed:survey_weight", source_column: "survey_weight", normalization: "none" },
    })).toThrowError(expect.objectContaining({ code: "schema.unknown_field" }));
    expect(() => parseResolvedWeightDeclarationV1({ ...declaration, inferred: true })).toThrowError(expect.objectContaining({ code: "schema.unknown_field" }));
  });

  it("emits the frozen unsupported code and copy for every current target", () => {
    const cases: Array<[SemWeightBindingV4, string, string]> = [
      [
        { kind: "case", variable: "observed:survey_weight" },
        "case_weight_unsupported",
        "Remove the case-weight binding or choose an estimator that explicitly supports case weights; no executable plan was emitted.",
      ],
      [
        { kind: "frequency", variable: "observed:survey_weight" },
        "frequency_weight_unsupported",
        "Remove the frequency-weight binding or choose an estimator that explicitly supports frequency weights; no executable plan was emitted.",
      ],
      [
        { kind: "sampling", variable: "observed:survey_weight", normalization: "mean_one" },
        "sampling_weight_unsupported",
        "Remove the sampling-weight binding or choose an estimator with explicit sampling-design support; no executable plan was emitted.",
      ],
    ];
    for (const target of targets) for (const [binding, code, correctiveAction] of cases) {
      const declaration = resolveSemWeightDeclarationV1(weightedModel(binding))!;
      const issue = weightCapabilityIssueV1(target, declaration);
      expect(issue).toEqual({
        code,
        target,
        declaration,
        subject: "observed:survey_weight",
        corrective_action: correctiveAction,
      });
      expect(parseWeightCapabilityIssueV1(JSON.parse(JSON.stringify(issue)))).toEqual(issue);
    }
  });

  it("parses the exact Rust JSON fixtures including an explicit legacy null", () => {
    const resolved = {
      code: "case_weight_unsupported",
      target: "pls_plan_v2",
      declaration: {
        contract_version: "sem_weight_declaration_v1",
        dataset_id: "dataset:survey",
        binding: {
          kind: "case",
          variable_id: "observed:weight",
          source_column: "survey_weight",
        },
      },
      subject: "observed:weight",
      corrective_action: "Remove the case-weight binding or choose an estimator that explicitly supports case weights; no executable plan was emitted.",
    };
    const legacy = {
      code: "legacy_case_weight_binding_ambiguous",
      target: "pls_plan_v2",
      declaration: null,
      subject: " case_weight ",
      corrective_action: "Legacy settings.case_weight_column ' case_weight ' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted.",
    };
    expect(parseWeightCapabilityIssueV1(JSON.parse(JSON.stringify(resolved)))).toEqual(resolved);
    expect(parseWeightCapabilityIssueV1(JSON.parse(JSON.stringify(legacy)))).toEqual(legacy);
    const whitespaceOnlyLegacy = {
      ...legacy,
      subject: "   ",
      corrective_action: "Legacy settings.case_weight_column '   ' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted.",
    };
    expect(parseWeightCapabilityIssueV1(JSON.parse(JSON.stringify(whitespaceOnlyLegacy)))).toEqual(whitespaceOnlyLegacy);
  });

  it("rejects code, declaration, subject, and corrective-copy drift at the native boundary", () => {
    const declaration = resolveSemWeightDeclarationV1(weightedModel({ kind: "case", variable: "observed:survey_weight" }))!;
    const issue = weightCapabilityIssueV1("pls_plan_v2", declaration);
    expect(() => parseWeightCapabilityIssueV1({ ...issue, code: "sampling_weight_unsupported" })).toThrowError(expect.objectContaining({ code: "schema.identity_mismatch" }));
    expect(() => parseWeightCapabilityIssueV1({ ...issue, subject: "observed:other" })).toThrowError(expect.objectContaining({ code: "schema.identity_mismatch" }));
    expect(() => parseWeightCapabilityIssueV1({ ...issue, corrective_action: "Try again." })).toThrowError(expect.objectContaining({ code: "schema.identity_mismatch" }));
    expect(() => parseWeightCapabilityIssueV1({
      ...issue,
      code: "legacy_case_weight_binding_ambiguous",
      subject: "survey_weight",
      corrective_action: "Legacy settings.case_weight_column 'survey_weight' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted.",
    })).toThrowError(expect.objectContaining({ code: "schema.identity_mismatch" }));
  });

  it("reserves normalization diagnostics and never substitutes them for a wholly unsupported sampling target", () => {
    const declaration = resolveSemWeightDeclarationV1(weightedModel({
      kind: "sampling",
      variable: "observed:survey_weight",
      normalization: "mean_one",
    }))!;
    expect(weightCapabilityIssueV1("cbsem_ml_v1", declaration).code).toBe("sampling_weight_unsupported");
    expect(samplingWeightNormalizationIssueV1("cbsem_ml_v1", declaration)).toMatchObject({
      code: "sampling_weight_normalization_unsupported",
      corrective_action: "Choose a supported sampling-weight normalization or remove the sampling-weight binding; no executable plan was emitted.",
    });
  });

  it("requires an exact SemModel case binding before translating a legacy case-weight column", () => {
    const declaration = resolveSemWeightDeclarationV1(weightedModel({ kind: "case", variable: "observed:survey_weight" }))!;
    expect(legacyCaseWeightCapabilityIssueV1("pls_plan_v2", "survey_weight", declaration).code).toBe("case_weight_unsupported");
    expect(legacyCaseWeightCapabilityIssueV1("pls_plan_v2", " survey_weight ", declaration).code).toBe("legacy_case_weight_binding_ambiguous");
    expect(legacyCaseWeightCapabilityIssueV1("pls_plan_v2", "other_weight", declaration)).toEqual({
      code: "legacy_case_weight_binding_ambiguous",
      target: "pls_plan_v2",
      declaration,
      subject: "other_weight",
      corrective_action: "Legacy settings.case_weight_column 'other_weight' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted.",
    });
    const unresolved = legacyCaseWeightCapabilityIssueV1("pls_plan_v2", "missing_weight", null);
    expect(unresolved).toEqual({
      code: "legacy_case_weight_binding_ambiguous",
      target: "pls_plan_v2",
      declaration: null,
      subject: "missing_weight",
      corrective_action: "Legacy settings.case_weight_column 'missing_weight' is not represented by an exact SemModelV4 case-weight binding to the same source column. Author that binding or clear the legacy setting; no executable plan was emitted.",
    });
    expect(JSON.stringify(unresolved)).toContain('"declaration":null');
    expect(parseWeightCapabilityIssueV1(JSON.parse(JSON.stringify(unresolved)))).toEqual(unresolved);
    const { declaration: _legacyDeclaration, ...legacyWithoutDeclaration } = unresolved;
    expect(() => parseWeightCapabilityIssueV1(legacyWithoutDeclaration)).toThrowError(SemWeightDeclarationV1Error);
    expect(() => legacyCaseWeightCapabilityIssueV1("pls_plan_v2", "", null)).toThrowError(expect.objectContaining({
      code: "legacy_case_weight_column_empty",
      subject: "settings.case_weight_column",
    }));
  });

  it("fails resolution when a binding does not identify a continuous observed control", () => {
    const model = weightedModel({ kind: "case", variable: "observed:survey_weight" });
    const weightVariable = model.variables.find((candidate) => candidate.id === "observed:survey_weight");
    if (weightVariable?.kind !== "observed") throw new Error("missing fixture weight variable");
    weightVariable.role = "indicator";
    expect(() => resolveSemWeightDeclarationV1(model)).toThrowError(expect.objectContaining({
      code: "model.invalid",
      subject: "observed:survey_weight",
    }));
  });

  it("rejects unknown diagnostic fields and absent declarations outside legacy ambiguity", () => {
    const declaration = resolveSemWeightDeclarationV1(weightedModel({ kind: "case", variable: "observed:survey_weight" })) as ResolvedWeightDeclarationV1;
    const issue = weightCapabilityIssueV1("cbsem_ml_v1", declaration);
    expect(() => parseWeightCapabilityIssueV1({ ...issue, executable: true })).toThrowError(expect.objectContaining({ code: "schema.unknown_field" }));
    const { declaration: _declaration, ...withoutDeclaration } = issue;
    expect(() => parseWeightCapabilityIssueV1(withoutDeclaration)).toThrowError(expect.objectContaining({
      code: "schema.invalid_shape",
      subject: "weight_issue.declaration",
    }));
  });
});
