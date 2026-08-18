import { describe, expect, it } from "vitest";
import {
  classifyInternalProjectArchiveSchemaV6,
  isExecutableProjectModelRecordV6Wire,
  parseInternalProjectArchiveV6Wire,
} from "./internalProjectArchiveV6Wire";
import {
  convertLegacyBasicModelV4,
  type LegacyBasicModelV4Input,
  type SemModelV4,
} from "./semModelV4";

const PROJECT_ID = "00000000-0000-0000-0000-000000000101";
const RECIPE_ID = "00000000-0000-0000-0000-000000000201";
const CURRENT_RECIPE_ID = "00000000-0000-0000-0000-000000000202";
const RESULT_ID = "00000000-0000-0000-0000-000000000301";
const SHA_A = "a".repeat(64);
const SHA_B = "b".repeat(64);

const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;

function lineage(sourceArchiveSchemaVersion = 3) {
  return {
    source_project_id: PROJECT_ID,
    source_archive_schema_version: sourceArchiveSchemaVersion,
    source_archive_sha256: SHA_A,
    source_archive_path: "C:\\source\\project.qpls",
    destination_archive_path: "D:\\upgraded\\project-v6.qpls",
    upgraded_at: "2026-08-15T09:00:00Z",
    source_preservation: "required",
    write_policy: "new_archive_only",
    historical_results_immutable: true,
  };
}

function baseArchive() {
  return {
    schema_version: 6,
    project_id: PROJECT_ID,
    name: "Schema v6 fixture",
    created_at: "2026-08-15T09:00:00Z",
    modified_at: "2026-08-15T09:01:00Z",
    origin: { kind: "new_project" },
  };
}

function legacySemInput(): LegacyBasicModelV4Input {
  return {
    id: "sem-model",
    name: "SEM model",
    constructs: [
      { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
      { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
    ],
    paths: [{ source: "x", target: "y" }],
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  };
}

function readyModel(): SemModelV4 {
  return convertLegacyBasicModelV4(legacySemInput(), "cbsem_common_factor");
}

function underidentifiedDraft(): SemModelV4 {
  const model = readyModel();
  const factor = model.variables.find((variable) => variable.kind === "common_factor");
  if (factor?.kind !== "common_factor") throw new Error("Expected a common-factor fixture variable.");
  factor.identification = { kind: "fixed_variance" };
  return model;
}

function recipeSettings() {
  return {
    method: "pls_pm",
    weighting_scheme: "path",
    tolerance: 1e-7,
    max_iterations: 3000,
    bootstrap_samples: 0,
    seed: 20260815,
  };
}

function historicalArchive() {
  return {
    ...baseArchive(),
    historical_recipes: [{
      recipe_id: RECIPE_ID,
      source_recipe_schema_version: 3,
      recipe_document: { schema_version: 3, id: RECIPE_ID, settings: { seed: 7 } },
      recipe_document_sha256: SHA_A,
    }],
    historical_results: [{
      result_id: RESULT_ID,
      source_result_schema_version: 3,
      result: { schema_version: 3, id: RESULT_ID, provenance: { recipe_id: RECIPE_ID } },
      result_sha256: SHA_B,
      source_recipe: {
        kind: "bound",
        source_recipe_id: RECIPE_ID,
        recipe_document_sha256: SHA_A,
      },
    }],
  };
}

function currentRecipe(id: string, model: SemModelV4) {
  return {
    schema_version: 4,
    id,
    created_at: "2026-08-15T09:02:00Z",
    dataset_fingerprint: "dataset-fingerprint",
    model_binding: { kind: "embedded_sem_model_v4", model, scientific_sha256: SHA_A },
    estimand_confirmation: "not_legacy",
    settings: recipeSettings(),
    method_config: { kind: "pls_algorithm" },
  };
}

describe("internal schema-v6 project archive wire", () => {
  it("classifies historical, current, and future schemas without interpreting future content", () => {
    expect(classifyInternalProjectArchiveSchemaV6(5)).toBe("historical_upgrade_copy_required");
    expect(classifyInternalProjectArchiveSchemaV6(6)).toBe("current_editable");
    expect(classifyInternalProjectArchiveSchemaV6(7)).toBe("future_read_only");
    expect(() => parseInternalProjectArchiveV6Wire({ schema_version: 7, unknown_future_shape: true }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.future_read_only" }));
  });

  it("applies Rust serde defaults to a new project without fabricating upgrade lineage", () => {
    const parsed = parseInternalProjectArchiveV6Wire(baseArchive());
    expect(parsed.origin).toEqual({ kind: "new_project" });
    expect(parsed).toMatchObject({
      datasets: [],
      models: [],
      recipes: [],
      historical_recipes: [],
      layouts: {},
      historical_results: [],
      canonical_result_documents: [],
    });
    expect(parsed).not.toHaveProperty("upgrade_lineage");

    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), datasets: null }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.array_required" }));
  });

  it("reads legacy top-level upgrade_lineage and emits only corrected upgraded_copy origin", () => {
    const legacy = { ...baseArchive(), upgrade_lineage: lineage() } as Record<string, unknown>;
    delete legacy.origin;

    const parsed = parseInternalProjectArchiveV6Wire(legacy);
    expect(parsed.origin).toEqual({ kind: "upgraded_copy", lineage: lineage() });
    expect(parsed).not.toHaveProperty("upgrade_lineage");
    expect(JSON.parse(JSON.stringify(parsed))).not.toHaveProperty("upgrade_lineage");

    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), upgrade_lineage: lineage() }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.origin_ambiguous" }));

    for (const field of ["source_archive_path", "destination_archive_path"] as const) {
      const slashOnly = lineage();
      slashOnly[field] = "///\\\\";
      const invalid = { ...baseArchive(), origin: { kind: "upgraded_copy", lineage: slashOnly } };
      expect(() => parseInternalProjectArchiveV6Wire(invalid))
        .toThrowError(expect.objectContaining({ code: "project_archive_v6.upgrade_path_empty" }));
    }
  });

  it("keeps SemModelV4 drafts non-executable and unavailable to current recipes", () => {
    const draft = underidentifiedDraft();
    const archive = {
      ...baseArchive(),
      models: [{
        model_id: draft.id,
        payload: { kind: "sem_model_v4_draft", model: draft, model_document_sha256: SHA_A },
      }],
    };
    const parsed = parseInternalProjectArchiveV6Wire(archive);
    expect(parsed.models[0].payload.kind).toBe("sem_model_v4_draft");
    expect(isExecutableProjectModelRecordV6Wire(parsed.models[0])).toBe(false);

    const recipeBindingDraft = clone(archive) as typeof archive & { recipes: unknown[] };
    recipeBindingDraft.recipes = [{
      schema_version: 4,
      id: CURRENT_RECIPE_ID,
      created_at: "2026-08-15T09:02:00Z",
      dataset_fingerprint: "dataset-fingerprint",
      model_binding: {
        kind: "project_sem_model_v4_reference",
        model_id: draft.id,
        scientific_sha256: SHA_A,
      },
      estimand_confirmation: "not_legacy",
      settings: recipeSettings(),
      method_config: { kind: "pls_algorithm" },
    }];
    expect(() => parseInternalProjectArchiveV6Wire(recipeBindingDraft))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.recipe_model_reference" }));
  });

  it("strictly decodes current RecipeV4 settings and the complete method-config wire boundary", () => {
    const recipe = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (recipe.settings as Record<string, unknown>).ignored_extension = "Rust drops this";
    const parsed = parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [recipe] });
    expect(parsed.recipes[0].settings).toEqual({
      ...recipeSettings(),
      studentized_inner_samples: 0,
      permutation_samples: 0,
      workers: 1,
      confidence_level: 0.95,
      preprocessing: "standardized",
      missing_data: "listwise_deletion",
      case_weight_column: null,
    });
    expect(parsed.recipes[0].method_config).toEqual({ kind: "pls_algorithm" });
    expect(parsed.recipes[0].metadata).toEqual({});

    const oneSided = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    oneSided.settings.bootstrap_samples = 999;
    (oneSided.settings as Record<string, unknown>).bootstrap_test_tail = "one_sided_less";
    (oneSided as { method_config: Record<string, unknown> }).method_config = { kind: "pls_bootstrap" };
    expect(parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [oneSided] }).recipes[0].settings)
      .toMatchObject({ bootstrap_samples: 999, bootstrap_test_tail: "one_sided_less" });

    const explicitDefault = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (explicitDefault.settings as Record<string, unknown>).bootstrap_test_tail = "two_sided";
    expect(parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [explicitDefault] }).recipes[0].settings)
      .not.toHaveProperty("bootstrap_test_tail");

    const wrongTail = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (wrongTail.settings as Record<string, unknown>).bootstrap_test_tail = "upper";
    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [wrongTail] }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.enum_invalid" }));

    const oneSidedPermutation = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (oneSidedPermutation as { method_config: Record<string, unknown> }).method_config = {
      kind: "plsc_permutation",
      group_column: "group",
      group_a: "A",
      group_b: "B",
      test_tail: "group_a_less",
    };
    expect(parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [oneSidedPermutation] }).recipes[0].method_config)
      .toEqual(oneSidedPermutation.method_config);

    const explicitPermutationDefault = structuredClone(oneSidedPermutation);
    (explicitPermutationDefault.method_config as Record<string, unknown>).test_tail = "two_sided";
    expect(parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [explicitPermutationDefault] }).recipes[0].method_config)
      .not.toHaveProperty("test_tail");

    const wrongPermutationTail = structuredClone(oneSidedPermutation);
    (wrongPermutationTail.method_config as Record<string, unknown>).test_tail = "upper";
    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [wrongPermutationTail] }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.enum_invalid" }));

    const cbsemTail = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    cbsemTail.settings.method = "cbsem";
    (cbsemTail as { method_config: Record<string, unknown> }).method_config = {
      kind: "cbsem", model_type: "cfa", estimator: "ml", input: "raw",
      mean_structure: false, bootstrap_samples: 1_000,
      bootstrap_v2: {
        algorithm: "case_resampling_full_ml", interval: "percentile_type7",
        test_tail: "one_sided_greater",
      },
    };
    expect(parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [cbsemTail] }).recipes[0].method_config)
      .toEqual(cbsemTail.method_config);

    const explicitCbsemDefault = structuredClone(cbsemTail);
    (explicitCbsemDefault.method_config as unknown as { bootstrap_v2: Record<string, unknown> }).bootstrap_v2.test_tail = "two_sided";
    expect((parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [explicitCbsemDefault] }).recipes[0].method_config as { bootstrap_v2: unknown }).bootstrap_v2)
      .not.toHaveProperty("test_tail");

    const wrongCbsemTail = structuredClone(cbsemTail);
    (wrongCbsemTail.method_config as unknown as { bootstrap_v2: Record<string, unknown> }).bootstrap_v2.test_tail = "upper";
    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [wrongCbsemTail] }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.enum_invalid" }));

    const regression = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    regression.settings.method = "regression";
    (regression as { method_config: Record<string, unknown> }).method_config = {
      kind: "regression",
      outcome: "y",
      predictors: ["x"],
      model: { type: "ols", robust_se: "hc3" },
      bootstrap: { algorithm: "case_resampling", intervals: ["percentile"] },
    };
    expect(parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [regression] }).recipes[0].method_config)
      .toEqual(regression.method_config);

    const nullDefault = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (nullDefault.settings as Record<string, unknown>).workers = null;
    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [nullDefault] }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.count_required" }));

    const invalidSettingsEnum = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (invalidSettingsEnum.settings as Record<string, unknown>).method = "permutation";
    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [invalidSettingsEnum] }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.enum_invalid" }));

    const invalidConfig = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (invalidConfig as { method_config: Record<string, unknown> }).method_config = { kind: "pls_algorithm", unexpected: true };
    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [invalidConfig] }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.field_unknown" }));

    const invalidNestedConfig = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (invalidNestedConfig as { method_config: Record<string, unknown> }).method_config = {
      kind: "cbsem",
      model_type: "sem",
      estimator: "magic",
      input: "raw",
      mean_structure: true,
      bootstrap_samples: 0,
    };
    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [invalidNestedConfig] }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.enum_invalid" }));

    const negativeZeroConfig = currentRecipe(CURRENT_RECIPE_ID, readyModel());
    (negativeZeroConfig as { method_config: Record<string, unknown> }).method_config = {
      kind: "cbsem",
      model_type: "sem",
      estimator: "ml",
      input: "raw",
      mean_structure: true,
      bootstrap_samples: -0,
    };
    expect(() => parseInternalProjectArchiveV6Wire({ ...baseArchive(), recipes: [negativeZeroConfig] }))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.u32_required" }));
  });

  it("normalizes omitted historical result bindings and enforces provenance-era binding rules", () => {
    const bound = parseInternalProjectArchiveV6Wire(historicalArchive());
    expect(bound.historical_results[0].source_recipe).toEqual({
      kind: "bound",
      source_recipe_id: RECIPE_ID,
      recipe_document_sha256: SHA_A,
    });

    const legacyWire = clone(historicalArchive());
    legacyWire.historical_recipes = [];
    delete (legacyWire.historical_results[0] as { source_recipe?: unknown }).source_recipe;
    expect(parseInternalProjectArchiveV6Wire(legacyWire).historical_results[0].source_recipe)
      .toEqual({ kind: "unbound_legacy" });

    const inventedUnbound = clone(historicalArchive());
    (inventedUnbound.historical_results[0] as { source_recipe: unknown }).source_recipe = { kind: "unbound_legacy" };
    expect(() => parseInternalProjectArchiveV6Wire(inventedUnbound))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.historical_result_recipe_binding" }));

    const schema2 = clone(inventedUnbound);
    (schema2 as { origin: unknown }).origin = { kind: "upgraded_copy", lineage: lineage(2) };
    expect(parseInternalProjectArchiveV6Wire(schema2).historical_results[0].source_recipe)
      .toEqual({ kind: "unbound_legacy" });

    const invalidSchema2Bound = clone(historicalArchive());
    (invalidSchema2Bound as { origin: unknown }).origin = { kind: "upgraded_copy", lineage: lineage(2) };
    expect(() => parseInternalProjectArchiveV6Wire(invalidSchema2Bound))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.historical_result_recipe_binding" }));
  });

  it("requires exact historical binding digests and disjoint current/historical recipe ids", () => {
    const digestMismatch = clone(historicalArchive());
    digestMismatch.historical_results[0].source_recipe.recipe_document_sha256 = "c".repeat(64);
    expect(() => parseInternalProjectArchiveV6Wire(digestMismatch))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.historical_result_recipe_binding" }));

    const duplicateRecipe = {
      ...historicalArchive(),
      recipes: [currentRecipe(RECIPE_ID, readyModel())],
    };
    expect(() => parseInternalProjectArchiveV6Wire(duplicateRecipe))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.recipe_id_duplicate" }));

    const historicalInCurrentLane = {
      ...baseArchive(),
      recipes: [{ ...currentRecipe(CURRENT_RECIPE_ID, readyModel()), schema_version: 3 }],
    };
    expect(() => parseInternalProjectArchiveV6Wire(historicalInCurrentLane))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.recipe_schema" }));
  });

  it("rejects old or mixed draft digest fields while leaving digest computation to Rust", () => {
    const draft = underidentifiedDraft();
    const wrongDigestField = {
      ...baseArchive(),
      models: [{
        model_id: draft.id,
        payload: { kind: "sem_model_v4_draft", model: draft, scientific_sha256: SHA_A },
      }],
    };
    expect(() => parseInternalProjectArchiveV6Wire(wrongDigestField))
      .toThrowError(expect.objectContaining({ code: "project_archive_v6.field_missing" }));

    const acceptedOpaqueDigest = clone(wrongDigestField);
    (acceptedOpaqueDigest.models[0] as { payload: unknown }).payload = {
      kind: "sem_model_v4_draft",
      model: draft,
      model_document_sha256: SHA_B,
    };
    expect(parseInternalProjectArchiveV6Wire(acceptedOpaqueDigest).models[0].payload)
      .toMatchObject({ kind: "sem_model_v4_draft", model_document_sha256: SHA_B });
  });
});
