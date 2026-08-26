import {
  INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1,
  INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_ACCESS_V1,
  type InternalProjectArchiveV6AccessV1,
} from "../domain/internalProjectArchiveV6Access";
import {
  getNativeMultiModCandidateAuthorityV1,
  parseNativeMultiModCandidateAuthorityV1,
} from "../native/nativeMultiModCandidateAuthorityV1";

export type InternalProjectArchiveV6CandidateAuthorityReaderV1 =
  () => Promise<unknown>;

export interface InternalProjectArchiveV6AccessDependenciesV1 {
  readonly candidateAuthority?: InternalProjectArchiveV6CandidateAuthorityReaderV1;
}

/**
 * Selects archive access from the executable's immutable candidate authority.
 * Invalid authority never falls back to the historical Labs surface.
 */
export async function resolveInternalProjectArchiveV6AccessV1(
  dependencies: InternalProjectArchiveV6AccessDependenciesV1 = {},
): Promise<InternalProjectArchiveV6AccessV1> {
  const authority = parseNativeMultiModCandidateAuthorityV1(
    await (dependencies.candidateAuthority ??
      (() => getNativeMultiModCandidateAuthorityV1()))(),
  );
  return authority.state === "release_qualified_candidate"
    ? INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_ACCESS_V1
    : INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1;
}
