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

type InterventionalRequestV1 = Extract<
  MultiModRecipeConfigV1,
  { kind: "interventional_causal_mediation_v1" }
>;

export function stageNativeInterventionalMediationV1(
  authority: NativeMultiModArchiveAuthorityV1,
  request: InterventionalRequestV1,
  access: NativeMultiModAccessV1,
  identity?: { readonly recipeId?: string; readonly createdAt?: string },
): NativeMultiModStagedRequestV1 {
  return stageNativeMultiModRequestV1(authority, request, access, identity);
}

export async function preflightNativeInterventionalMediationV1(
  request: NativeMultiModStagedRequestV1,
): Promise<NativeMultiModPreflightV1> {
  if (request.config.kind !== "interventional_causal_mediation_v1")
    throw new Error(
      "The interventional-mediation V1 adapter refuses a different MultiMod target.",
    );
  const preflight = await preflightNativeMultiModJobV1(request);
  if (preflight.target !== "interventional_causal_mediation_v1")
    throw new Error(
      "Native interventional-mediation preflight returned a different target.",
    );
  return preflight;
}

export async function startNativeInterventionalMediationV1(
  request: NativeMultiModStagedRequestV1,
) {
  const preflight = await preflightNativeInterventionalMediationV1(request);
  if (preflight.readiness !== "built_in_from_dataset")
    throw new Error(
      `Interventional mediation V1 is not natively executable: ${preflight.stableReasonCodes.join(", ")}`,
    );
  const job = await startNativeMultiModJobV1(request);
  if (job.target !== "interventional_causal_mediation_v1")
    throw new Error(
      "Native interventional-mediation start returned a different target.",
    );
  return job;
}

export async function getNativeInterventionalMediationResultV1(
  jobId: string,
): Promise<NativeMultiModCompletedResultV1> {
  const completed = await getNativeMultiModJobResultV1(jobId);
  if (completed.attachment.result.kind !== "interventional_mediation_result_v1")
    throw new Error(
      "Native interventional-mediation result returned a different result family.",
    );
  return completed;
}
