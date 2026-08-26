import { describe, expect, it, vi } from "vitest";
import {
  getNativeMultiModCandidateAuthorityV1,
  parseNativeMultiModCandidateAuthorityV1,
  selectNativeMultiModWorkspaceAccessV1,
} from "./nativeMultiModCandidateAuthorityV1";

const labsAuthority = {
  schemaVersion: 1,
  state: "labs_only",
  standardSurfaceAuthorized: false,
  embeddedDocumentSha256: "a".repeat(64),
  authorityBindingSha256: null,
  candidateCommitSha: null,
  candidateVersion: null,
  qualificationPlanSha256: null,
  gateBindingSha256: null,
  capabilityIndexSha256: null,
  prepackageManifestSetSha256: null,
  exactProfileCells: [],
} as const;

const releaseAuthority = {
  schemaVersion: 1,
  state: "release_qualified_candidate",
  standardSurfaceAuthorized: true,
  embeddedDocumentSha256: "a".repeat(64),
  authorityBindingSha256: "b".repeat(64),
  candidateCommitSha: "c".repeat(40),
  candidateVersion: "2.56.0",
  qualificationPlanSha256: "d".repeat(64),
  gateBindingSha256: "e".repeat(64),
  capabilityIndexSha256: "f".repeat(64),
  prepackageManifestSetSha256: "1".repeat(64),
  exactProfileCells: [
    "conditional.multi_two_way_percentile.v2::explicit_path_target_math",
    "mga.general_sem_pls.v1::point_estimation",
  ],
} as const;

describe("native MultiMod candidate authority", () => {
  it("reads the immutable native authority and selects Standard independently of Labs preference", async () => {
    const invokeNative = vi.fn().mockResolvedValue(releaseAuthority);
    const authority = await getNativeMultiModCandidateAuthorityV1(
      invokeNative as unknown as typeof import("@tauri-apps/api/core").invoke,
    );

    expect(invokeNative).toHaveBeenCalledWith(
      "multimod_candidate_authority_status_v1",
    );
    expect(selectNativeMultiModWorkspaceAccessV1(authority, false)).toMatchObject(
      {
        access: {
          surface: "standard_multimod_v1",
          experimentalLabsEnabled: false,
        },
        displayLabel: "Standard · Release-qualified",
      },
    );
  });

  it("keeps the Labs sentinel fail-closed unless the user explicitly opts in", () => {
    const authority = parseNativeMultiModCandidateAuthorityV1(labsAuthority);
    expect(selectNativeMultiModWorkspaceAccessV1(authority, false)).toBeNull();
    expect(selectNativeMultiModWorkspaceAccessV1(authority, true)).toMatchObject({
      access: {
        surface: "internal_labs_multimod_v1",
        experimentalLabsEnabled: true,
      },
      displayLabel: "Experimental Labs · Unqualified",
    });
  });

  it("rejects malformed, widened, or internally inconsistent native authority", () => {
    expect(() =>
      parseNativeMultiModCandidateAuthorityV1({
        ...labsAuthority,
        standardSurfaceAuthorized: true,
      }),
    ).toThrow(/cannot carry candidate authority data/u);
    expect(() =>
      parseNativeMultiModCandidateAuthorityV1({
        ...releaseAuthority,
        fabricated: true,
      }),
    ).toThrow(/unsupported versioned shape/u);
    expect(() =>
      parseNativeMultiModCandidateAuthorityV1({
        ...releaseAuthority,
        exactProfileCells: [...releaseAuthority.exactProfileCells].reverse(),
      }),
    ).toThrow(/sorted and unique/u);
    expect(() =>
      parseNativeMultiModCandidateAuthorityV1({
        ...releaseAuthority,
        exactProfileCells: ["mga.*::point_estimation"],
      }),
    ).toThrow(/wildcard-free/u);
  });
});
