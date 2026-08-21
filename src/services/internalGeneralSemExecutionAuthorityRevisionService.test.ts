import { describe, expect, it, vi } from "vitest";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import { capabilityRegistryV2 } from "../domain/capabilityRegistryV2";
import { defaultGeneralSemConfigV1 } from "../domain/generalSemConfigV1";
import {
  reviseInternalGeneralSemExecutionAuthorityAtV1,
  selectGeneralSemRevisionExecutionV1,
} from "./internalGeneralSemExecutionAuthorityRevisionService";

const sha = (value: string) => value.repeat(64);
const sourceProjectId = "10000000-0000-4000-8000-000000000001";
const sourceRecipeId = "10000000-0000-4000-8000-000000000002";
const projectId = "10000000-0000-4000-8000-000000000003";
const recipeId = "10000000-0000-4000-8000-000000000004";
const datasetId = "10000000-0000-4000-8000-000000000005";
const moderationPointCell = {
  registry_schema_version: 2 as const,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
  capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1",
};

function request() {
  return {
    surface: "internal_labs" as const,
    experimentalLabsEnabled: true as const,
    sourceArchivePath: "D:\\source.qpls",
    expectedSourceArchiveSha256: sha("a"),
    destinationArchivePath: "D:\\revision.qpls",
    revision: {
      source: { projectId: sourceProjectId, modelId: "model:source", modelDocumentSha256: sha("b"), modelScientificSha256: sha("c"), recipeId: sourceRecipeId, recipeDocumentSha256: sha("d") },
      revision: { projectId, projectName: "Revision", createdAt: "2026-08-19T10:00:00Z", modelId: "model:revision", modelName: "Revision model", recipeId },
      intent: { kind: "add_general_sem_interaction_v2" as const, intent_version: 1 as const, sem_generation: "general_sem_v1" as const, label: "X × W", operands: ["x", "w"] as const, focal_relation: "path:x-y", outcome: "y", method: "two_stage" as const, hierarchy_policy: "strong" as const },
      expectedCapabilityCell: moderationPointCell,
      recipeExecutionSurface: "native_general_sem_pls_labs_v1" as const,
    },
  };
}

function receipt() {
  return {
    schemaVersion: 1, archiveSchemaVersion: 6, revisionNumber: 1,
    sourceArchivePath: "D:\\source.qpls", sourceArchiveSha256: sha("a"), sourceArchiveBytes: 100, sourceVerifiedUnchanged: true,
    sourceProjectId, sourceModelId: "model:source", sourceModelDocumentSha256: sha("b"), sourceModelScientificSha256: sha("c"), sourceRecipeId, sourceRecipeDocumentSha256: sha("d"),
    destinationArchivePath: "D:\\revision.qpls", destinationArchiveSha256: sha("e"), destinationArchiveBytes: 200, strictReopenValidated: true,
    projectId, name: "Revision", createdAt: "2026-08-19T10:00:00Z", residentDatasetId: datasetId, residentDatasetFingerprint: "dataset-fingerprint",
    residentModelId: "model:revision", residentModelDocumentSha256: sha("f"), residentModelScientificSha256: sha("0"), residentRecipeId: recipeId, residentRecipeDocumentSha256: sha("1"),
    compilerVersion: "compiler-v1", capabilityCell: moderationPointCell,
    recipeAnalyticalSha256: sha("2"), generalSemConfigSha256: sha("3"), compiledPlanSha256: sha("4"), compiledArtifactIdentitySha256: sha("5"),
    interactionTermId: "general-sem:v1:interaction:path%3Ax-y:x:w",
    interactionOutputId: "general-sem:v1:interaction-output:general-sem%3Av1%3Ainteraction%3Apath%253Ax-y%3Ax%3Aw",
  };
}

function snapshot(): InternalProjectArchiveV6ReadSnapshotV1 {
  const r = receipt();
  return {
    schemaVersion: 1, access: "read_only", loader: "strict_schema6_zip",
    archivePath: r.destinationArchivePath, archiveSha256: r.destinationArchiveSha256, archiveBytes: r.destinationArchiveBytes,
    manifest: { schema_version: 6, project_id: projectId, name: "Revision", created_at: r.createdAt, modified_at: r.createdAt, engine_version: "test", checksum_algorithm: "sha256", checksums: {} },
    project: {
      schema_version: 6, project_id: projectId, name: "Revision", created_at: r.createdAt, modified_at: r.createdAt,
      datasets: [{ id: datasetId, name: "Data", fingerprint: "dataset-fingerprint", schema: { version: 1, kind: "raw", case_count: 10, sample_size: 10, columns: [] } }],
      models: [{
        model_id: "model:revision",
        payload: {
          kind: "sem_model_v4",
          model: {
            variables: [],
            relations: [],
            parameters: [],
            constraints: [],
            derived_terms: [],
            annotations: [],
          } as never,
          scientific_sha256: sha("0"),
        },
      }],
      recipes: [{} as never], historical_recipes: [], layouts: { general_sem_execution_authority_revision_v1: { schemaVersion: 1, revisionNumber: 1, revised: { projectId, modelDocumentSha256: sha("f") }, compilation: { compiledArtifactIdentitySha256: sha("5") } } }, historical_results: [], canonical_result_documents: [], origin: { kind: "new_project" }, sem_generation: "general_sem_v1",
    },
    residentDatasets: [{ datasetId, name: "Data", fingerprint: "dataset-fingerprint", rowCount: 10, columnCount: 0, sampleSize: 10, arrowResident: true }],
    counts: { datasets: 1, models: 1, recipes: 1, historicalRecipes: 0, historicalResults: 0, canonicalResultDocuments: 0 },
    generalSemExecutionAuthority: {
      schemaVersion: 1, projectId, datasetId, datasetFingerprint: "dataset-fingerprint",
      modelId: "model:revision", modelScientificSha256: sha("0"), recipeId,
      recipeDocumentSha256: sha("1"),
      recipe: { metadata: { execution_surface: "native_general_sem_pls_labs_v1" } } as never,
    },
    sourceRecheckedUnchanged: true,
  };
}

describe("General SEM revision persistence service", () => {
  it("selects point versus supplemental bootstrap revision cells from Registry authority", () => {
    const standardRegistry = {
      requireOptionCell(capabilityId: string, cellId: string) {
        return {
          ...capabilityRegistryV2.requireOptionCell(capabilityId, cellId),
          surface: "standard" as const,
          coverage_state: "partial" as const,
          evidence_state: "release_qualified" as const,
        };
      },
    };
    const point = snapshot();
    point.generalSemExecutionAuthority!.recipe = {
      general_sem_config: defaultGeneralSemConfigV1(),
      metadata: { execution_surface: "native_general_sem_pls_labs_v1" },
    } as never;
    const moderationIntent = request().revision.intent;
    expect(selectGeneralSemRevisionExecutionV1({
      snapshot: point,
      intent: moderationIntent,
      experimentalLabsEnabled: false,
      capabilityRegistry: standardRegistry,
    })).toMatchObject({
      access: { surface: "standard", experimentalLabsEnabled: false },
      expectedCapabilityCell: moderationPointCell,
      recipeExecutionSurface: "native_general_sem_pls_standard_v1",
    });

    expect(selectGeneralSemRevisionExecutionV1({
      snapshot: point,
      intent: {
        kind: "add_higher_order",
        term_id: "term:hoc",
        output_id: "derived:hoc",
        label: "HOC",
        components: ["construct:a", "construct:b"],
        approach: "disjoint_two_stage",
        measurement_type: "reflective_reflective",
        initial_path: {
          relation_id: "relation:hoc_y",
          source: "derived:hoc",
          target: "construct:y",
          label: "HOC effect",
        },
      },
      experimentalLabsEnabled: false,
      capabilityRegistry: standardRegistry,
    }).expectedCapabilityCell.cell_id).toBe("qpls3.pls.general_sem_higher_order_point");

    const hocSource = structuredClone(point);
    const hocPayload = hocSource.project.models[0]?.payload;
    if (!hocPayload || hocPayload.kind !== "sem_model_v4") throw new Error("HOC test requires SemModelV4.");
    hocPayload.model.variables = [{ kind: "derived", id: "derived:hoc", label: "HOC" }];
    hocPayload.model.derived_terms = [{
      kind: "higher_order",
      id: "term:hoc",
      output: "derived:hoc",
      components: ["construct:a", "construct:b"],
      approach: "disjoint_two_stage",
      measurement_type: "reflective_reflective",
    }];
    expect(selectGeneralSemRevisionExecutionV1({
      snapshot: hocSource,
      intent: {
        kind: "remove_higher_order",
        term_id: "term:hoc",
        output_id: "derived:hoc",
      },
      experimentalLabsEnabled: false,
      capabilityRegistry: standardRegistry,
    }).expectedCapabilityCell.cell_id).toBe("qpls3.pls.mediation");

    const bootstrap = structuredClone(point);
    bootstrap.generalSemExecutionAuthority!.recipe.general_sem_config = {
      ...defaultGeneralSemConfigV1(),
      inference: {
        kind: "case_bootstrap", resamples: 500, seed: 7,
        confidence_level: 0.95, interval: "percentile", tail: "two_sided",
      },
    };
    expect(selectGeneralSemRevisionExecutionV1({
      snapshot: bootstrap,
      intent: moderationIntent,
      experimentalLabsEnabled: false,
      capabilityRegistry: standardRegistry,
    }).expectedCapabilityCell.cell_id)
      .toBe("qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap");

    const labsRegistry = {
      requireOptionCell(capabilityId: string, cellId: string) {
        return {
          ...capabilityRegistryV2.requireOptionCell(capabilityId, cellId),
          surface: "labs" as const,
          coverage_state: "partial" as const,
          evidence_state: "archive_qualified" as const,
        };
      },
    };
    expect(() => selectGeneralSemRevisionExecutionV1({
      snapshot: point,
      intent: moderationIntent,
      experimentalLabsEnabled: false,
      capabilityRegistry: labsRegistry,
    })).toThrowError(expect.objectContaining({
      code: "general_sem.access.experimental_labs_required",
    }));
  });

  it("accepts only a strict destination snapshot matching every receipt identity", async () => {
    const invokeNative = vi.fn(async () => ({ status: "ok", value: { schemaVersion: 1, persistence: "persisted_new_revision", receipt: receipt() } }));
    const inspectDestination = vi.fn(async () => ({ status: "ok", value: snapshot() } as const));
    const outcome = await reviseInternalGeneralSemExecutionAuthorityAtV1(request(), { invokeNative: invokeNative as never, inspectDestination });
    expect(outcome.status).toBe("ok");
    expect(invokeNative).toHaveBeenCalledWith("revise_internal_general_sem_execution_authority_v1", { request: request() });
  });

  it("reports a persisted-but-not-activated revision when strict client inspection disagrees", async () => {
    const bad = snapshot();
    bad.generalSemExecutionAuthority = { ...bad.generalSemExecutionAuthority!, modelScientificSha256: sha("9") };
    const outcome = await reviseInternalGeneralSemExecutionAuthorityAtV1(request(), {
      invokeNative: vi.fn(async () => ({ status: "ok", value: { schemaVersion: 1, persistence: "persisted_new_revision", receipt: receipt() } })) as never,
      inspectDestination: vi.fn(async () => ({ status: "ok", value: bad } as const)),
    });
    expect(outcome.status).toBe("blocked");
    if (outcome.status === "blocked") {
      expect(outcome.persistedReceipt?.destinationArchivePath).toBe("D:\\revision.qpls");
      expect(outcome.diagnostic.code).toContain("destination_authority_mismatch");
    }
  });
});
