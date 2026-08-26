import { describe, expect, it } from "vitest";
import { resolveInternalProjectArchiveV6AccessV1 } from "./internalProjectArchiveV6AccessService";

function candidateAuthority(qualified: boolean) {
  return {
    schemaVersion: 1,
    state: qualified ? "release_qualified_candidate" : "labs_only",
    standardSurfaceAuthorized: qualified,
    embeddedDocumentSha256: "a".repeat(64),
    authorityBindingSha256: qualified ? "b".repeat(64) : null,
    candidateCommitSha: qualified ? "c".repeat(40) : null,
    candidateVersion: qualified ? "2.56.0" : null,
    qualificationPlanSha256: qualified ? "d".repeat(64) : null,
    gateBindingSha256: qualified ? "e".repeat(64) : null,
    capabilityIndexSha256: qualified ? "f".repeat(64) : null,
    prepackageManifestSetSha256: qualified ? "1".repeat(64) : null,
    exactProfileCells: qualified
      ? ["mga.general_sem_pls.v1::point_estimation"]
      : [],
  };
}

describe("schema-6 archive candidate-authority access", () => {
  it("maps qualified authority to Standard and the sentinel to historical Labs", async () => {
    await expect(
      resolveInternalProjectArchiveV6AccessV1({
        candidateAuthority: async () => candidateAuthority(true),
      }),
    ).resolves.toEqual({
      surface: "standard_multimod_v1",
      experimentalLabsEnabled: false,
    });
    await expect(
      resolveInternalProjectArchiveV6AccessV1({
        candidateAuthority: async () => candidateAuthority(false),
      }),
    ).resolves.toEqual({
      surface: "internal_labs",
      experimentalLabsEnabled: true,
    });
  });

  it("does not downgrade malformed embedded authority to Labs", async () => {
    await expect(
      resolveInternalProjectArchiveV6AccessV1({
        candidateAuthority: async () => ({
          ...candidateAuthority(true),
          standardSurfaceAuthorized: false,
        }),
      }),
    ).rejects.toThrow(/binding is incomplete/u);
  });
});
