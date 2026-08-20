import { describe, expect, it } from "vitest";
import {
  GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1,
  saveInternalGeneralSemModeratedMediationRevisionV1,
} from "./internalGeneralSemModeratedMediationRevisionV2Service";

describe("internal moderated-mediation revision service boundary", () => {
  it("keeps the product route closed and performs no persistence", async () => {
    expect(GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1).toBe(false);

    const outcome = await saveInternalGeneralSemModeratedMediationRevisionV1({
      status: "ready",
      candidates: [],
      selectedPathId: "sem_specific_path_v1_test",
      selectedPath: {
        pathId: "sem_specific_path_v1_test",
        estimandId: "sem_specific_path_v1_test",
        orderedRelationIds: ["relation:x:m", "relation:m:y"],
        xId: "x",
        xLabel: "X",
        mediatorId: "m",
        mediatorLabel: "M",
        yId: "y",
        yLabel: "Y",
        moderatorId: "w",
        moderatorLabel: "W",
        interactionId: "interaction:x:w",
        moderatedStage: "first_stage",
        moderatedRelationId: "relation:x:m",
        otherStageRelationId: "relation:m:y",
      },
      autoSelected: true,
      targetInventory: [] as never,
      supplementalCapabilityCell: {} as never,
      capabilityDependencies: [],
      revisedConfig: {} as never,
      issues: [],
    });

    expect(outcome).toEqual({
      status: "blocked",
      diagnostic: expect.objectContaining({
        code: "general_sem.moderated_mediation.native_revision_not_connected",
      }),
    });
  });
});
