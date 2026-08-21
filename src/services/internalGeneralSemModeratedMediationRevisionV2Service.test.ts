import { describe, expect, it, vi } from "vitest";
import {
  destinationSnapshotV2,
  receiptV2,
  requestV2,
  selectionV1,
  transactionV2,
} from "../domain/internalGeneralSemModeratedMediationRevisionV2.testFixtures";
import {
  GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1,
  reviseInternalGeneralSemModeratedMediationAtV2,
  saveInternalGeneralSemModeratedMediationRevisionV1,
} from "./internalGeneralSemModeratedMediationRevisionV2Service";

describe("internal moderated-mediation revision service boundary", () => {
  it("opens the Registry-authorized Standard route and cancels before persistence", async () => {
    expect(GENERAL_SEM_MODERATED_MEDIATION_PRODUCT_ROUTE_CONNECTED_V1).toBe(true);
    const transaction = transactionV2();
    const invokeNative = vi.fn();
    const outcome = await saveInternalGeneralSemModeratedMediationRevisionV1({
      snapshot: transaction.snapshot,
      source: transaction.source,
      revision: transaction.revision,
      selection: selectionV1(),
      revisionNumberHint: 1,
    }, {
      selectDestination: vi.fn(async () => null),
      invokeNative,
    });

    expect(outcome).toBeNull();
    expect(invokeNative).not.toHaveBeenCalled();
  });

  it("uses the exact native v2 command and returns only a strictly reopened transaction", async () => {
    const invokeNative = vi.fn(async () => ({
      status: "ok",
      value: {
        schemaVersion: 2,
        persistence: "persisted_new_revision",
        receipt: receiptV2(),
      },
    }));
    const inspectDestination = vi.fn(async () => ({
      status: "ok",
      value: destinationSnapshotV2(),
    } as const));

    const outcome = await reviseInternalGeneralSemModeratedMediationAtV2(transactionV2(), {
      invokeNative,
      inspectDestination,
    });

    expect(outcome.status).toBe("ok");
    expect(invokeNative).toHaveBeenCalledWith(
      "revise_internal_general_sem_execution_authority_v2",
      { request: requestV2() },
    );
    expect(inspectDestination).toHaveBeenCalledWith("D:\\revision-v2.qpls");
  });

  it("fails closed when native transport rejects and never claims persistence", async () => {
    const inspectDestination = vi.fn();
    const outcome = await reviseInternalGeneralSemModeratedMediationAtV2(transactionV2(), {
      invokeNative: vi.fn(async () => { throw new Error("transport rejected"); }),
      inspectDestination,
    });

    expect(outcome.status).toBe("blocked");
    if (outcome.status === "blocked") {
      expect(outcome.diagnostic.code).toContain("native_transport_failed");
      expect(outcome.persistedReceipt).toBeUndefined();
    }
    expect(inspectDestination).not.toHaveBeenCalled();
  });

  it("returns a recovery receipt without replacing the source after post-persist inspection failure", async () => {
    const transaction = transactionV2();
    const sourceBefore = structuredClone(transaction.snapshot);
    const outcome = await reviseInternalGeneralSemModeratedMediationAtV2(transaction, {
      invokeNative: vi.fn(async () => ({
        status: "ok",
        value: {
          schemaVersion: 2,
          persistence: "persisted_new_revision",
          receipt: receiptV2(),
        },
      })),
      inspectDestination: vi.fn(async () => { throw new Error("strict reopen unavailable"); }),
    });

    expect(outcome.status).toBe("blocked");
    if (outcome.status === "blocked") {
      expect(outcome.diagnostic.code).toContain("destination_inspection_failed");
      expect(outcome.persistedReceipt?.destinationArchivePath).toBe("D:\\revision-v2.qpls");
      expect(outcome.diagnostic.correctiveAction).toContain("open that exact saved revision explicitly");
    }
    expect(transaction.snapshot).toEqual(sourceBefore);
  });

  it("stops after persistence when the source session changes and does not inspect or activate", async () => {
    const current = vi.fn()
      .mockReturnValueOnce(true)
      .mockReturnValueOnce(false);
    const inspectDestination = vi.fn();
    const outcome = await reviseInternalGeneralSemModeratedMediationAtV2(transactionV2(), {
      invokeNative: vi.fn(async () => ({
        status: "ok",
        value: {
          schemaVersion: 2,
          persistence: "persisted_new_revision",
          receipt: receiptV2(),
        },
      })),
      inspectDestination,
      isSourceSessionCurrent: current,
    });

    expect(outcome.status).toBe("blocked");
    if (outcome.status === "blocked") {
      expect(outcome.diagnostic.code).toContain("source_session_changed_after_persist");
      expect(outcome.persistedReceipt).toEqual(receiptV2());
    }
    expect(inspectDestination).not.toHaveBeenCalled();
  });
});
