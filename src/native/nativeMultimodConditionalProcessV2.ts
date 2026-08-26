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

type ConditionalRequestV2 = Extract<
  MultiModRecipeConfigV1,
  { kind: "general_sem_conditional_process_v2" }
>;

export function stageNativeMultimodConditionalProcessV2(
  authority: NativeMultiModArchiveAuthorityV1,
  request: ConditionalRequestV2,
  access: NativeMultiModAccessV1,
  identity?: { readonly recipeId?: string; readonly createdAt?: string },
): NativeMultiModStagedRequestV1 {
  return stageNativeMultiModRequestV1(authority, request, access, identity);
}

export async function preflightNativeMultimodConditionalProcessV2(
  request: NativeMultiModStagedRequestV1,
): Promise<NativeMultiModPreflightV1> {
  if (request.config.kind !== "general_sem_conditional_process_v2")
    throw new Error(
      "The conditional-process V2 adapter refuses a different MultiMod target.",
    );
  const preflight = await preflightNativeMultiModJobV1(request);
  if (preflight.target !== "general_sem_conditional_process_v2")
    throw new Error(
      "Native conditional-process preflight returned a different target.",
    );
  return preflight;
}

export async function startNativeMultimodConditionalProcessV2(
  request: NativeMultiModStagedRequestV1,
) {
  const preflight = await preflightNativeMultimodConditionalProcessV2(request);
  if (preflight.readiness !== "built_in_from_dataset")
    throw new Error(
      `Conditional-process V2 remains fail-closed: ${preflight.stableReasonCodes.join(", ")}`,
    );
  const job = await startNativeMultiModJobV1(request);
  if (job.target !== "general_sem_conditional_process_v2")
    throw new Error(
      "Native conditional-process start returned a different target.",
    );
  return job;
}

export async function getNativeMultimodConditionalProcessResultV2(
  jobId: string,
): Promise<NativeMultiModCompletedResultV1> {
  const completed = await getNativeMultiModJobResultV1(jobId);
  if (
    completed.attachment.result.kind !==
    "general_sem_conditional_process_result_v2"
  )
    throw new Error(
      "Native conditional-process result returned a different result family.",
    );
  return completed;
}
