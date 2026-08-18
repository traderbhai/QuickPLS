import { invoke } from "@tauri-apps/api/core";
import {
  parseStandardSemModelV4AuthorityCasOutcomeV1,
  parseStandardSemModelV4AuthorityCasRequestV1,
  parseStandardSemModelV4AuthorityResolveOutcomeV1,
  parseStandardSemModelV4AuthorityResolveRequestV1,
  type StandardSemModelV4AuthorityCasOutcomeV1,
  type StandardSemModelV4AuthorityResolveOutcomeV1,
} from "../domain/standardSemModelV4AuthorityCas";
import type { SemModelV4 } from "../domain/semModelV4";

const STANDARD_SEM_MODEL_V4_AUTHORITY_CAS_COMMAND =
  "compare_and_swap_standard_sem_model_v4_authority";
const STANDARD_SEM_MODEL_V4_AUTHORITY_RESOLVE_COMMAND =
  "resolve_standard_sem_model_v4_authority";

/** Resolves the first native document digest for one complete detached model. */
export async function resolveStandardSemModelV4Authority(
  model: SemModelV4,
): Promise<StandardSemModelV4AuthorityResolveOutcomeV1> {
  const request = parseStandardSemModelV4AuthorityResolveRequestV1({ model });
  const response = await invoke<unknown>(STANDARD_SEM_MODEL_V4_AUTHORITY_RESOLVE_COMMAND, {
    request,
  });
  return parseStandardSemModelV4AuthorityResolveOutcomeV1(response, request);
}

/**
 * Native document-digest CAS for one complete Standard SemModelV4 authority.
 *
 * It is stateless and has no graph, project, save, or persistence side effect.
 * Native `blocked` outcomes return normally; transport and wire-contract errors
 * reject so callers can preserve their current authority unchanged.
 */
export async function compareAndSwapStandardSemModelV4Authority(
  sourceModel: SemModelV4,
  expectedSourceModelDocumentSha256: string,
  candidate: SemModelV4,
): Promise<StandardSemModelV4AuthorityCasOutcomeV1> {
  const request = parseStandardSemModelV4AuthorityCasRequestV1({
    expectedSourceModelDocumentSha256,
    sourceModel,
    candidate,
  });
  const response = await invoke<unknown>(STANDARD_SEM_MODEL_V4_AUTHORITY_CAS_COMMAND, {
    request,
  });
  return parseStandardSemModelV4AuthorityCasOutcomeV1(response, request);
}
