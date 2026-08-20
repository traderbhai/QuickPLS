import { describe, expect, it } from "vitest";
import {
  parseGeneralSemExecutionAuthorityRevisionLineageV2,
  parseInternalGeneralSemModeratedMediationRevisionNativeOutcomeV2,
  parseInternalGeneralSemModeratedMediationRevisionRequestV2,
} from "./internalGeneralSemModeratedMediationRevisionV2";
import {
  compiledTargetV1,
  lineageV2,
  receiptV2,
  requestV2,
} from "./internalGeneralSemModeratedMediationRevisionV2.testFixtures";

describe("internal moderated-mediation revision-v2 wire", () => {
  it("strictly reconciles request, receipt, full compiled target, and lineage", () => {
    const request = parseInternalGeneralSemModeratedMediationRevisionRequestV2(requestV2());
    const outcome = parseInternalGeneralSemModeratedMediationRevisionNativeOutcomeV2({
      status: "ok",
      value: {
        schemaVersion: 2,
        persistence: "persisted_new_revision",
        receipt: receiptV2(),
      },
    }, request);
    expect(outcome.status).toBe("ok");
    if (outcome.status !== "ok") return;
    const lineage = parseGeneralSemExecutionAuthorityRevisionLineageV2(
      lineageV2(),
      request,
      outcome.value.receipt,
    );
    expect(lineage.compiledTarget).toEqual(compiledTargetV1());
    expect(lineage.parentRevisionNumber).toBe(0);
  });

  it("rejects a native target that substitutes the gamma-only moderation bootstrap cell", () => {
    const receipt = receiptV2();
    receipt.compiledTarget.bootstrap_capability_cell = {
      registry_schema_version: 2,
      capability_id: "smartpls.moderation",
      cell_id: "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
      capability_version: "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
    };
    expect(() => parseInternalGeneralSemModeratedMediationRevisionNativeOutcomeV2({
      status: "ok",
      value: { schemaVersion: 2, persistence: "persisted_new_revision", receipt },
    }, requestV2())).toThrow(/bounded moderated-mediation contract/);
  });

  it("rejects lineage target or digest tampering after a valid native receipt", () => {
    const lineage = lineageV2();
    lineage.compiledTarget.moderated_relation_id = lineage.compiledTarget.other_stage_relation_id;
    expect(() => parseGeneralSemExecutionAuthorityRevisionLineageV2(
      lineage,
      requestV2(),
      receiptV2(),
    )).toThrow(/bounded moderated-mediation contract|lineage differs/);

    const digestTampered = lineageV2();
    digestTampered.compiledTargetSha256 = "9".repeat(64);
    expect(() => parseGeneralSemExecutionAuthorityRevisionLineageV2(
      digestTampered,
      requestV2(),
      receiptV2(),
    )).toThrow(/lineage differs/);
  });

  it("rejects unknown response fields and stale selected-path identities", () => {
    expect(() => parseInternalGeneralSemModeratedMediationRevisionNativeOutcomeV2({
      status: "ok",
      value: {
        schemaVersion: 2,
        persistence: "persisted_new_revision",
        receipt: { ...receiptV2(), unexpected: true },
      },
    }, requestV2())).toThrow(/not allowed/);

    const request = requestV2();
    request.revision.intent.estimand_id = "sem_specific_path_v1_stale";
    expect(() => parseInternalGeneralSemModeratedMediationRevisionRequestV2(request))
      .toThrow(/stable selected-path identity/);
  });
});
