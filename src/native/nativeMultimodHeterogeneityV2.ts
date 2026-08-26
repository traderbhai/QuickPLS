import type { MultiModRecipeConfigV1 } from "../domain/multimodContractsV1";
import {
  getNativeMultiModJobResultV1,
  preflightNativeMultiModJobV1,
  stageNativeMultiModRequestV1,
  startNativeMultiModJobV1,
  type NativeMultiModArchiveAuthorityV1,
  type NativeMultiModAccessV1,
  type NativeMultiModCompletedResultV1,
  type NativeMultiModPreflightV1,
  type NativeMultiModStagedRequestV1,
} from "./nativeMultiModJobV1";

type HeterogeneityRequestV2 = Extract<
  MultiModRecipeConfigV1,
  { kind: "pls_unobserved_heterogeneity_v2" }
>;

export function stageNativeMultimodHeterogeneityV2(
  authority: NativeMultiModArchiveAuthorityV1,
  request: HeterogeneityRequestV2,
  access: NativeMultiModAccessV1,
  identity?: { readonly recipeId?: string; readonly createdAt?: string },
): NativeMultiModStagedRequestV1 {
  return stageNativeMultiModRequestV1(authority, request, access, identity);
}

export async function preflightNativeMultimodHeterogeneityV2(
  request: NativeMultiModStagedRequestV1,
): Promise<NativeMultiModPreflightV1> {
  if (request.config.kind !== "pls_heterogeneity_v2")
    throw new Error(
      "The heterogeneity V2 adapter refuses a different MultiMod target.",
    );
  const preflight = await preflightNativeMultiModJobV1(request);
  if (preflight.target !== "pls_heterogeneity_v2")
    throw new Error(
      "Native heterogeneity preflight returned a different target.",
    );
  return preflight;
}

export async function startNativeMultimodHeterogeneityV2(
  request: NativeMultiModStagedRequestV1,
) {
  const preflight = await preflightNativeMultimodHeterogeneityV2(request);
  if (preflight.readiness !== "built_in_from_dataset")
    throw new Error(
      `Heterogeneity V2 is not natively executable: ${preflight.stableReasonCodes.join(", ")}`,
    );
  const job = await startNativeMultiModJobV1(request);
  if (job.target !== "pls_heterogeneity_v2")
    throw new Error("Native heterogeneity start returned a different target.");
  return job;
}

export async function getNativeMultimodHeterogeneityResultV2(
  jobId: string,
): Promise<NativeMultiModCompletedResultV1> {
  const completed = await getNativeMultiModJobResultV1(jobId);
  if (completed.attachment.result.kind !== "pls_heterogeneity_analysis_v2")
    throw new Error(
      "Native heterogeneity result returned a different result family.",
    );
  return completed;
}
