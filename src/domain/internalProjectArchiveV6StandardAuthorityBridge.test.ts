import { describe, expect, it } from "vitest";
import {
  deriveInternalProjectArchiveV6StandardSaveCandidateV1,
  internalProjectArchiveV6ScientificEditLockedModelIdsV1,
  InternalProjectArchiveV6StandardAuthorityBridgeError,
  type InternalProjectArchiveV6StandardSaveAuthorityV1,
} from "./internalProjectArchiveV6StandardAuthorityBridge";
import {
  parseInternalProjectArchiveV6Wire,
  type InternalProjectArchiveV6Wire,
} from "./internalProjectArchiveV6Wire";
import {
  convertLegacyBasicModelV4,
  type LegacyBasicModelV4Input,
  type SemModelV4,
} from "./semModelV4";
import { parseStandardSemModelV4AuthorityRecordV1 } from "./standardSemModelV4Authority";
import {
  parseStandardSemModelV4DiagramLayoutV1,
  projectStandardSemModelV4DiagramV1,
} from "./standardSemModelV4DiagramProjection";

const SCIENTIFIC_SHA_A = "a".repeat(64);
const SCIENTIFIC_SHA_B = "b".repeat(64);
const DOCUMENT_SHA_A = "c".repeat(64);
const DOCUMENT_SHA_B = "d".repeat(64);

const legacyModel: LegacyBasicModelV4Input = {
  id: "standard-recipe-model",
  name: "Recipe-bound model",
  constructs: [
    { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
    { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
  ],
  paths: [{ source: "x", target: "y" }],
  controls: [],
  higher_order_constructs: [],
  interactions: [],
};

function recipeSettings() {
  return {
    method: "pls_pm",
    weighting_scheme: "path",
    tolerance: 1e-7,
    max_iterations: 3000,
    bootstrap_samples: 0,
    studentized_inner_samples: 0,
    permutation_samples: 0,
    seed: 20260815,
    workers: 1,
    confidence_level: 0.95,
    preprocessing: "standardized",
    missing_data: "listwise_deletion",
    case_weight_column: null,
  };
}

function sourceProject(): InternalProjectArchiveV6Wire {
  const model = convertLegacyBasicModelV4(legacyModel, "cbsem_common_factor");
  return parseInternalProjectArchiveV6Wire({
    schema_version: 6,
    project_id: "00000000-0000-0000-0000-000000000701",
    name: "Recipe dependency fixture",
    created_at: "2026-08-15T10:00:00Z",
    modified_at: "2026-08-15T10:01:00Z",
    origin: { kind: "new_project" },
    models: [{
      model_id: model.id,
      payload: {
        kind: "sem_model_v4",
        model,
        scientific_sha256: SCIENTIFIC_SHA_A,
      },
    }],
    recipes: [{
      schema_version: 4,
      id: "00000000-0000-0000-0000-000000000702",
      created_at: "2026-08-15T10:02:00Z",
      dataset_fingerprint: "dataset-fingerprint",
      model_binding: {
        kind: "project_sem_model_v4_reference",
        model_id: model.id,
        scientific_sha256: SCIENTIFIC_SHA_A,
      },
      estimand_confirmation: "not_legacy",
      settings: recipeSettings(),
      method_config: { kind: "pls_algorithm" },
      metadata: { purpose: "dependency-guard" },
    }],
    layouts: { preserved_lane: { opaque: "unchanged" } },
  });
}

function sourceModel(source: InternalProjectArchiveV6Wire): SemModelV4 {
  const payload = source.models[0]?.payload;
  if (!payload || payload.kind === "legacy_estimand_unspecified") {
    throw new Error("Expected a SemModelV4 source fixture.");
  }
  return payload.model;
}

function saveAuthority(
  source: InternalProjectArchiveV6Wire,
  overrides: Partial<Pick<InternalProjectArchiveV6StandardSaveAuthorityV1, "readiness" | "scientificSha256">> = {},
  model: SemModelV4 = sourceModel(source),
  documentSha256 = DOCUMENT_SHA_A,
): InternalProjectArchiveV6StandardSaveAuthorityV1 {
  const authority = parseStandardSemModelV4AuthorityRecordV1({
    schema_version: 1,
    model_document_sha256: documentSha256,
    model,
  });
  const projected = projectStandardSemModelV4DiagramV1(authority);
  return {
    authority,
    layout: parseStandardSemModelV4DiagramLayoutV1({
      schema_version: 1,
      model_id: model.id,
      diagram_layout: projected.diagramLayout,
    }),
    readiness: "ready",
    scientificSha256: SCIENTIFIC_SHA_A,
    ...overrides,
  };
}

describe("schema-6 Standard authority RecipeV4 dependency guard", () => {
  it("rejects a changed ready scientific revision before a referenced RecipeV4 can become stale", () => {
    const source = sourceProject();
    const sourceBefore = JSON.stringify(source);
    const changedModel = { ...sourceModel(source), name: "Changed scientific model" };
    const current = saveAuthority(
      source,
      { readiness: "ready", scientificSha256: SCIENTIFIC_SHA_B },
      changedModel,
      DOCUMENT_SHA_B,
    );

    let thrown: unknown;
    try {
      deriveInternalProjectArchiveV6StandardSaveCandidateV1(source, {
        [changedModel.id]: current,
      });
    } catch (error) {
      thrown = error;
    }

    expect(thrown).toBeInstanceOf(InternalProjectArchiveV6StandardAuthorityBridgeError);
    expect(thrown).toEqual(expect.objectContaining({
      name: "InternalProjectArchiveV6StandardAuthorityBridgeError",
      code: "schema6_standard_bridge.recipe_model_reference_stale",
      subject: "project.recipes[0].model_binding",
      correctiveAction: expect.stringContaining("will not rewrite or discard the recipe binding"),
    }));
    expect(JSON.stringify(source)).toBe(sourceBefore);
    expect(source.recipes[0].model_binding).toEqual({
      kind: "project_sem_model_v4_reference",
      model_id: changedModel.id,
      scientific_sha256: SCIENTIFIC_SHA_A,
    });
  });

  it("allows an unchanged ready authority and preserves all recipe and result lanes by reference", () => {
    const source = sourceProject();
    const current = saveAuthority(source);
    const candidate = deriveInternalProjectArchiveV6StandardSaveCandidateV1(source, {
      [current.authority.model.id]: current,
    });

    expect(candidate.recipes).toBe(source.recipes);
    expect(candidate.historical_recipes).toBe(source.historical_recipes);
    expect(candidate.historical_results).toBe(source.historical_results);
    expect(candidate.canonical_result_documents).toBe(source.canonical_result_documents);
    expect(candidate.recipes[0].model_binding).toBe(source.recipes[0].model_binding);
    expect(candidate.layouts.preserved_lane).toBe(source.layouts.preserved_lane);
    expect(candidate.models[0].payload).toMatchObject({
      kind: "sem_model_v4",
      scientific_sha256: SCIENTIFIC_SHA_A,
    });
  });

  it("allows layout-only changes without rebinding the ready RecipeV4 digest", () => {
    const source = sourceProject();
    const current = saveAuthority(source);
    current.layout = parseStandardSemModelV4DiagramLayoutV1({
      ...current.layout,
      diagram_layout: {
        ...current.layout.diagram_layout,
        diagramViewport: { x: 40, y: 50, zoom: 1.2 },
      },
    });

    const candidate = deriveInternalProjectArchiveV6StandardSaveCandidateV1(source, {
      [current.authority.model.id]: current,
    });

    expect(candidate.recipes).toBe(source.recipes);
    expect(candidate.recipes[0].model_binding).toEqual({
      kind: "project_sem_model_v4_reference",
      model_id: current.authority.model.id,
      scientific_sha256: SCIENTIFIC_SHA_A,
    });
    expect(candidate.layouts.standard_sem_model_v4_diagram_layouts_v1).toEqual({
      schema_version: 1,
      models: {
        [current.authority.model.id]: current.layout,
      },
    });
  });

  it("persists an additional unbound authority while retaining the old RecipeV4 binding", () => {
    const source = sourceProject();
    const oldAuthority = saveAuthority(source);
    const revisionModel = { ...sourceModel(source), id: "standard-recipe-model:revision:2", name: "Revision 2" };
    const extended: InternalProjectArchiveV6Wire = {
      ...source,
      models: [...source.models, {
        model_id: revisionModel.id,
        payload: { kind: "sem_model_v4", model: revisionModel, scientific_sha256: SCIENTIFIC_SHA_B },
      }],
    };
    const revisionAuthority = saveAuthority(
      extended,
      { readiness: "ready", scientificSha256: SCIENTIFIC_SHA_B },
      revisionModel,
      DOCUMENT_SHA_B,
    );
    const candidate = deriveInternalProjectArchiveV6StandardSaveCandidateV1(extended, {
      [oldAuthority.authority.model.id]: oldAuthority,
      [revisionModel.id]: revisionAuthority,
    });

    expect(candidate.models).toHaveLength(2);
    expect(internalProjectArchiveV6ScientificEditLockedModelIdsV1(extended))
      .toEqual([source.models[0].model_id]);
    expect(candidate.recipes).toBe(source.recipes);
    expect(candidate.historical_recipes).toBe(source.historical_recipes);
    expect(candidate.historical_results).toBe(source.historical_results);
    expect(candidate.canonical_result_documents).toBe(source.canonical_result_documents);
    expect(candidate.recipes[0].model_binding).toEqual({
      kind: "project_sem_model_v4_reference",
      model_id: source.models[0].model_id,
      scientific_sha256: SCIENTIFIC_SHA_A,
    });
  });
});
