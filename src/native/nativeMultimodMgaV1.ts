import type { MultiModRecipeConfigV1 } from "../domain/multimodContractsV1";
import {
  getNativeMultiModJobResultV1,
  preflightNativeMultiModJobV1,
  stageNativeMultiModRequestV1,
  startNativeMultiModJobV1,
  type NativeMultiModArchiveAuthorityV1,
  type NativeMultiModCompletedResultV1,
  type NativeMultiModPreflightV1,
  type NativeMultiModStagedRequestV1,
} from "./nativeMultiModJobV1";

type MgaRequestV1 = Extract<
  MultiModRecipeConfigV1,
  { kind: "mga_multigroup_v1" }
>;

export function stageNativeMultimodMgaV1(
  authority: NativeMultiModArchiveAuthorityV1,
  request: MgaRequestV1,
  identity?: { readonly recipeId?: string; readonly createdAt?: string },
): NativeMultiModStagedRequestV1 {
  return stageNativeMultiModRequestV1(authority, request, identity);
}

export async function preflightNativeMultimodMgaV1(
  request: NativeMultiModStagedRequestV1,
): Promise<NativeMultiModPreflightV1> {
  if (request.config.kind !== "mga_multigroup_v1")
    throw new Error("The MGA V1 adapter refuses a different MultiMod target.");
  const preflight = await preflightNativeMultiModJobV1(request);
  if (preflight.target !== "mga_multigroup_v1")
    throw new Error("Native MGA preflight returned a different target.");
  return preflight;
}

export async function startNativeMultimodMgaV1(
  request: NativeMultiModStagedRequestV1,
) {
  const preflight = await preflightNativeMultimodMgaV1(request);
  if (preflight.readiness !== "built_in_from_dataset")
    throw new Error(
      `MGA is not natively executable: ${preflight.stableReasonCodes.join(", ")}`,
    );
  const job = await startNativeMultiModJobV1(request);
  if (job.target !== "mga_multigroup_v1")
    throw new Error("Native MGA start returned a different target.");
  return job;
}

export async function getNativeMultimodMgaResultV1(
  jobId: string,
): Promise<NativeMultiModCompletedResultV1> {
  const completed = await getNativeMultiModJobResultV1(jobId);
  if (completed.attachment.result.kind !== "pls_multigroup_analysis_v1")
    throw new Error("Native MGA result returned a different result family.");
  return completed;
}
