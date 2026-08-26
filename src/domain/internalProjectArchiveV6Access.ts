export const INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1 = Object.freeze({
  surface: "internal_labs",
  experimentalLabsEnabled: true,
} as const);

export const INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_ACCESS_V1 = Object.freeze({
  surface: "standard_multimod_v1",
  experimentalLabsEnabled: false,
} as const);

export type InternalProjectArchiveV6AccessV1 =
  | typeof INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1
  | typeof INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_ACCESS_V1;

/** Returns only the two native-authorized surface/flag pairs. */
export function internalProjectArchiveV6AccessPairV1(
  surface: unknown,
  experimentalLabsEnabled: unknown,
): InternalProjectArchiveV6AccessV1 | null {
  if (surface === "internal_labs" && experimentalLabsEnabled === true) {
    return INTERNAL_PROJECT_ARCHIVE_V6_LABS_ACCESS_V1;
  }
  if (
    surface === "standard_multimod_v1" &&
    experimentalLabsEnabled === false
  ) {
    return INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_ACCESS_V1;
  }
  return null;
}
