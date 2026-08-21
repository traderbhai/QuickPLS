import { describe, expect, it } from "vitest";
import {
  parseInternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1,
  parseInternalGeneralSemExecutionAuthorityRevisionRequestV1,
} from "./internalGeneralSemExecutionAuthorityRevisionV1";

const sourceProjectId = "10000000-0000-4000-8000-000000000001";
const sourceRecipeId = "10000000-0000-4000-8000-000000000002";
const projectId = "10000000-0000-4000-8000-000000000003";
const recipeId = "10000000-0000-4000-8000-000000000004";
const sha = (character: string) => character.repeat(64);
const moderationPointCell = {
  registry_schema_version: 2 as const,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_point",
  capability_version: "general_sem_pls_multiple_two_way_moderation_point_v1",
};
const moderationBootstrapCell = {
  registry_schema_version: 2 as const,
  capability_id: "smartpls.moderation",
  cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
  capability_version: "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
};

function request() {
  return parseInternalGeneralSemExecutionAuthorityRevisionRequestV1({
    surface: "internal_labs",
    experimentalLabsEnabled: true,
    sourceArchivePath: "D:\\source.qpls",
    expectedSourceArchiveSha256: sha("a"),
    destinationArchivePath: "D:\\revision.qpls",
    revision: {
      source: {
        projectId: sourceProjectId,
        modelId: "model:source",
        modelDocumentSha256: sha("b"),
        modelScientificSha256: sha("c"),
        recipeId: sourceRecipeId,
        recipeDocumentSha256: sha("d"),
      },
      revision: {
        projectId,
        projectName: "Study revision 1",
        createdAt: "2026-08-19T10:00:00.000Z",
        modelId: "model:revision:1",
        modelName: "Model revision 1",
        recipeId,
      },
      intent: {
        kind: "add_general_sem_interaction_v2",
        intent_version: 1,
        sem_generation: "general_sem_v1",
        label: "X × W",
        operands: ["x", "w"],
        focal_relation: "path:x-y",
        outcome: "y",
        method: "two_stage",
        hierarchy_policy: "strong",
      },
      expectedCapabilityCell: moderationPointCell,
      recipeExecutionSurface: "native_general_sem_pls_labs_v1",
    },
  });
}

describe("General SEM execution-authority revision v1 wire", () => {
  it("accepts only the exact versioned Labs request", () => {
    const parsed = request();
    expect(parsed.revision.intent.operands).toEqual(["x", "w"]);
    expect(() => parseInternalGeneralSemExecutionAuthorityRevisionRequestV1({
      ...parsed,
      revision: { ...parsed.revision, intent: { ...parsed.revision.intent, kind: "add_interaction" } },
    })).toThrow(/exact General SEM interaction-v2 intent/);
    expect(() => parseInternalGeneralSemExecutionAuthorityRevisionRequestV1({
      ...parsed,
      destinationArchivePath: parsed.sourceArchivePath,
    })).toThrow(/new destination path/);
  });

  it("pins every authority and deterministic interaction identity in the receipt", () => {
    const parsedRequest = request();
    const receipt = {
      schemaVersion: 1,
      archiveSchemaVersion: 6,
      revisionNumber: 1,
      sourceArchivePath: parsedRequest.sourceArchivePath,
      sourceArchiveSha256: parsedRequest.expectedSourceArchiveSha256,
      sourceArchiveBytes: 123,
      sourceVerifiedUnchanged: true,
      sourceProjectId,
      sourceModelId: "model:source",
      sourceModelDocumentSha256: sha("b"),
      sourceModelScientificSha256: sha("c"),
      sourceRecipeId,
      sourceRecipeDocumentSha256: sha("d"),
      destinationArchivePath: parsedRequest.destinationArchivePath,
      destinationArchiveSha256: sha("e"),
      destinationArchiveBytes: 456,
      strictReopenValidated: true,
      projectId,
      name: "Study revision 1",
      createdAt: "2026-08-19T10:00:00.000Z",
      residentDatasetId: "10000000-0000-4000-8000-000000000005",
      residentDatasetFingerprint: "dataset-fingerprint",
      residentModelId: "model:revision:1",
      residentModelDocumentSha256: sha("f"),
      residentModelScientificSha256: sha("0"),
      residentRecipeId: recipeId,
      residentRecipeDocumentSha256: sha("1"),
      compilerVersion: "general-sem-pls-v1",
      capabilityCell: moderationPointCell,
      recipeAnalyticalSha256: sha("2"),
      generalSemConfigSha256: sha("3"),
      compiledPlanSha256: sha("4"),
      compiledArtifactIdentitySha256: sha("5"),
      interactionTermId: "general-sem:v1:interaction:path%3Ax-y:x:w",
      interactionOutputId: "general-sem:v1:interaction-output:general-sem%3Av1%3Ainteraction%3Apath%253Ax-y%3Ax%3Aw",
    };
    const parsed = parseInternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1({
      status: "ok",
      value: { schemaVersion: 1, persistence: "persisted_new_revision", receipt },
    }, parsedRequest);
    expect(parsed.status).toBe("ok");
    if (parsed.status === "ok") expect(parsed.value.receipt.revisionNumber).toBe(1);
    const bootstrapRequest = parseInternalGeneralSemExecutionAuthorityRevisionRequestV1({
      ...parsedRequest,
      revision: {
        ...parsedRequest.revision,
        expectedCapabilityCell: moderationBootstrapCell,
      },
    });
    expect(parseInternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1({
      status: "ok",
      value: { schemaVersion: 1, persistence: "persisted_new_revision", receipt },
    }, bootstrapRequest)).toMatchObject({
      status: "ok",
      value: { receipt: { capabilityCell: moderationPointCell } },
    });
    expect(() => parseInternalGeneralSemExecutionAuthorityRevisionNativeOutcomeV1({
      status: "ok",
      value: { schemaVersion: 1, persistence: "persisted_new_revision", receipt: { ...receipt, sourceModelScientificSha256: sha("9") } },
    }, parsedRequest)).toThrow(/differs from the exact pinned request/);
  });
});
