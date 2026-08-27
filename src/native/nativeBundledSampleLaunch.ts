import {
  BUNDLED_SAMPLE_PROJECTS,
  type NativeSampleProjectId,
} from "../domain/bundledSampleCatalog";

export interface NativeBundledSampleLaunchDependenciesV1 {
  openOrdinarySample: (sampleId: NativeSampleProjectId) => Promise<void>;
  materializeGeneralSemSample: (sampleId: NativeSampleProjectId) => Promise<string>;
  openGeneralSemArchive: (archivePath: string) => Promise<void>;
}

export type NativeBundledSampleLaunchOutcomeV1 = "ordinary_v1" | "general_sem_v1";

/**
 * Routes one catalog-backed sample through its declared persistence authority.
 * A strict General SEM sample is materialized as schema 6 and can never fall
 * back to the ordinary demo-project command.
 */
export async function launchNativeBundledSampleV1(
  sampleId: NativeSampleProjectId,
  dependencies: NativeBundledSampleLaunchDependenciesV1,
): Promise<NativeBundledSampleLaunchOutcomeV1> {
  const sample = BUNDLED_SAMPLE_PROJECTS.find((candidate) => candidate.id === sampleId);
  if (!sample) {
    throw new Error(`Bundled sample ${JSON.stringify(sampleId)} is not present in the validated catalog.`);
  }
  if (sample.projectKind === "general_sem_v1") {
    const archivePath = await dependencies.materializeGeneralSemSample(sampleId);
    await dependencies.openGeneralSemArchive(archivePath);
    return "general_sem_v1";
  }
  await dependencies.openOrdinarySample(sampleId);
  return "ordinary_v1";
}
