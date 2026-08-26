import { invoke } from "@tauri-apps/api/core";
import {
  NATIVE_MULTIMOD_LABS_ACCESS_V1,
  NATIVE_MULTIMOD_STANDARD_ACCESS_V1,
  type NativeMultiModAccessV1,
} from "./nativeMultiModJobV1";

const COMMAND = "multimod_candidate_authority_status_v1";
const SHA1 = /^[a-f0-9]{40}$/u;
const SHA256 = /^[a-f0-9]{64}$/u;

interface NativeMultiModCandidateAuthorityBaseV1 {
  readonly schemaVersion: 1;
  readonly standardSurfaceAuthorized: boolean;
  readonly embeddedDocumentSha256: string;
}

export interface NativeMultiModLabsOnlyAuthorityV1
  extends NativeMultiModCandidateAuthorityBaseV1 {
  readonly state: "labs_only";
  readonly standardSurfaceAuthorized: false;
  readonly authorityBindingSha256: null;
  readonly candidateCommitSha: null;
  readonly candidateVersion: null;
  readonly qualificationPlanSha256: null;
  readonly gateBindingSha256: null;
  readonly capabilityIndexSha256: null;
  readonly prepackageManifestSetSha256: null;
  readonly exactProfileCells: readonly [];
}

export interface NativeMultiModReleaseQualifiedAuthorityV1
  extends NativeMultiModCandidateAuthorityBaseV1 {
  readonly state: "release_qualified_candidate";
  readonly standardSurfaceAuthorized: true;
  readonly authorityBindingSha256: string;
  readonly candidateCommitSha: string;
  readonly candidateVersion: string;
  readonly qualificationPlanSha256: string;
  readonly gateBindingSha256: string;
  readonly capabilityIndexSha256: string;
  readonly prepackageManifestSetSha256: string;
  readonly exactProfileCells: readonly string[];
}

export type NativeMultiModCandidateAuthorityV1 =
  | NativeMultiModLabsOnlyAuthorityV1
  | NativeMultiModReleaseQualifiedAuthorityV1;

export interface NativeMultiModWorkspaceAccessV1 {
  readonly access: NativeMultiModAccessV1;
  readonly authority: NativeMultiModCandidateAuthorityV1;
  readonly displayLabel:
    | "Standard · Release-qualified"
    | "Experimental Labs · Unqualified";
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("The native MultiMod candidate authority must be an object.");
  }
  return value as Record<string, unknown>;
}

function exactRecord(value: unknown): Record<string, unknown> {
  const item = record(value);
  const keys = [
    "schemaVersion",
    "state",
    "standardSurfaceAuthorized",
    "embeddedDocumentSha256",
    "authorityBindingSha256",
    "candidateCommitSha",
    "candidateVersion",
    "qualificationPlanSha256",
    "gateBindingSha256",
    "capabilityIndexSha256",
    "prepackageManifestSetSha256",
    "exactProfileCells",
  ] as const;
  if (
    Object.keys(item).length !== keys.length ||
    keys.some((key) => !Object.prototype.hasOwnProperty.call(item, key))
  ) {
    throw new Error(
      "The native MultiMod candidate authority has an unsupported versioned shape.",
    );
  }
  return item;
}

function sha256(value: unknown, path: string): string {
  if (typeof value !== "string" || !SHA256.test(value)) {
    throw new Error(`${path} must be a lowercase SHA-256.`);
  }
  return value;
}

function exactProfileCell(value: unknown, path: string): string {
  if (
    typeof value !== "string" ||
    !/^[a-z0-9][a-z0-9._-]*::[a-z0-9][a-z0-9._-]*$/u.test(value)
  ) {
    throw new Error(`${path} must be one wildcard-free profile::procedure identity.`);
  }
  return value;
}

export function parseNativeMultiModCandidateAuthorityV1(
  value: unknown,
): NativeMultiModCandidateAuthorityV1 {
  const item = exactRecord(value);
  if (item.schemaVersion !== 1 || !Array.isArray(item.exactProfileCells)) {
    throw new Error("The native MultiMod candidate authority schema is unsupported.");
  }
  const embeddedDocumentSha256 = sha256(
    item.embeddedDocumentSha256,
    "candidateAuthority.embeddedDocumentSha256",
  );
  if (item.state === "labs_only") {
    if (
      item.standardSurfaceAuthorized !== false ||
      item.authorityBindingSha256 !== null ||
      item.candidateCommitSha !== null ||
      item.candidateVersion !== null ||
      item.qualificationPlanSha256 !== null ||
      item.gateBindingSha256 !== null ||
      item.capabilityIndexSha256 !== null ||
      item.prepackageManifestSetSha256 !== null ||
      item.exactProfileCells.length !== 0
    ) {
      throw new Error(
        "The Labs-only MultiMod sentinel cannot carry candidate authority data.",
      );
    }
    return {
      schemaVersion: 1,
      state: "labs_only",
      standardSurfaceAuthorized: false,
      embeddedDocumentSha256,
      authorityBindingSha256: null,
      candidateCommitSha: null,
      candidateVersion: null,
      qualificationPlanSha256: null,
      gateBindingSha256: null,
      capabilityIndexSha256: null,
      prepackageManifestSetSha256: null,
      exactProfileCells: [],
    };
  }
  if (item.state !== "release_qualified_candidate") {
    throw new Error("The native MultiMod candidate authority state is unsupported.");
  }
  if (
    item.standardSurfaceAuthorized !== true ||
    typeof item.candidateCommitSha !== "string" ||
    !SHA1.test(item.candidateCommitSha) ||
    typeof item.candidateVersion !== "string" ||
    !item.candidateVersion ||
    item.candidateVersion.trim() !== item.candidateVersion ||
    item.exactProfileCells.length === 0
  ) {
    throw new Error("The release-qualified MultiMod authority binding is incomplete.");
  }
  const exactProfileCells = item.exactProfileCells.map((cell, index) =>
    exactProfileCell(cell, `candidateAuthority.exactProfileCells[${index}]`),
  );
  if (
    exactProfileCells.some(
      (cell, index) => index > 0 && exactProfileCells[index - 1] >= cell,
    )
  ) {
    throw new Error(
      "The release-qualified MultiMod profile cells must be sorted and unique.",
    );
  }
  return {
    schemaVersion: 1,
    state: "release_qualified_candidate",
    standardSurfaceAuthorized: true,
    embeddedDocumentSha256,
    authorityBindingSha256: sha256(
      item.authorityBindingSha256,
      "candidateAuthority.authorityBindingSha256",
    ),
    candidateCommitSha: item.candidateCommitSha,
    candidateVersion: item.candidateVersion,
    qualificationPlanSha256: sha256(
      item.qualificationPlanSha256,
      "candidateAuthority.qualificationPlanSha256",
    ),
    gateBindingSha256: sha256(
      item.gateBindingSha256,
      "candidateAuthority.gateBindingSha256",
    ),
    capabilityIndexSha256: sha256(
      item.capabilityIndexSha256,
      "candidateAuthority.capabilityIndexSha256",
    ),
    prepackageManifestSetSha256: sha256(
      item.prepackageManifestSetSha256,
      "candidateAuthority.prepackageManifestSetSha256",
    ),
    exactProfileCells,
  };
}

export async function getNativeMultiModCandidateAuthorityV1(
  invokeNative: typeof invoke = invoke,
): Promise<NativeMultiModCandidateAuthorityV1> {
  return parseNativeMultiModCandidateAuthorityV1(
    await invokeNative<unknown>(COMMAND),
  );
}

export function selectNativeMultiModWorkspaceAccessV1(
  authority: NativeMultiModCandidateAuthorityV1,
  experimentalLabsEnabled: boolean,
): NativeMultiModWorkspaceAccessV1 | null {
  if (authority.state === "release_qualified_candidate") {
    return {
      access: NATIVE_MULTIMOD_STANDARD_ACCESS_V1,
      authority,
      displayLabel: "Standard · Release-qualified",
    };
  }
  return experimentalLabsEnabled
    ? {
        access: NATIVE_MULTIMOD_LABS_ACCESS_V1,
        authority,
        displayLabel: "Experimental Labs · Unqualified",
      }
    : null;
}
