import { invoke } from "@tauri-apps/api/core";
import {
  INTERNAL_PROJECT_ARCHIVE_V6_MODEL_MUTATION_SURFACE,
  parseInternalProjectArchiveV6ModelMutationOutcomeV1,
  parseInternalProjectArchiveV6ModelMutationRequestV1,
  type InternalProjectArchiveV6ModelMutationV1,
} from "../domain/internalProjectArchiveV6ModelMutation";
import type { InternalProjectArchiveV6Wire } from "../domain/internalProjectArchiveV6Wire";
import type { SemModelV4 } from "../domain/semModelV4";
import type { StandardSemModelV4AuthorityResolveResultV1 } from "../domain/standardSemModelV4AuthorityCas";

const MUTATE_SCHEMA6_MODEL_COMMAND = "mutate_internal_project_archive_v6_model";

/**
 * Applies one pure schema-6 model mutation to an ephemeral document.
 *
 * The native command has no file path, active-project store, save, autosave,
 * recovery, or Standard-workspace integration.
 */
export async function mutateInternalProjectArchiveV6Model(
  project: InternalProjectArchiveV6Wire,
  mutation: InternalProjectArchiveV6ModelMutationV1,
) {
  const request = parseInternalProjectArchiveV6ModelMutationRequestV1({
    surface: INTERNAL_PROJECT_ARCHIVE_V6_MODEL_MUTATION_SURFACE,
    experimentalLabsEnabled: true,
    project,
    mutation,
  });
  const response = await invoke<unknown>(MUTATE_SCHEMA6_MODEL_COMMAND, { request });
  return parseInternalProjectArchiveV6ModelMutationOutcomeV1(response, request);
}

export function insertInternalProjectArchiveV6ModelDraft(
  project: InternalProjectArchiveV6Wire,
  draft: SemModelV4,
) {
  return mutateInternalProjectArchiveV6Model(project, { kind: "insert_draft", draft });
}

export function replaceInternalProjectArchiveV6ModelDraft(
  project: InternalProjectArchiveV6Wire,
  modelId: string,
  expectedModelDocumentSha256: string,
  replacement: SemModelV4,
) {
  return mutateInternalProjectArchiveV6Model(project, {
    kind: "replace_draft",
    modelId,
    expectedModelDocumentSha256,
    replacement,
  });
}

export function promoteInternalProjectArchiveV6ModelDraft(
  project: InternalProjectArchiveV6Wire,
  modelId: string,
  expectedModelDocumentSha256: string,
) {
  return mutateInternalProjectArchiveV6Model(project, {
    kind: "promote_draft",
    modelId,
    expectedModelDocumentSha256,
  });
}

export class InternalProjectArchiveV6ModelRevisionAppendError extends Error {
  constructor(public readonly code: string, message: string, public readonly correctiveAction: string) {
    super(message);
    this.name = "InternalProjectArchiveV6ModelRevisionAppendError";
  }
}

const revisionAppendFailure = (code: string, message: string, correctiveAction: string): never => {
  throw new InternalProjectArchiveV6ModelRevisionAppendError(code, message, correctiveAction);
};

/** Appends one native-resolved ready revision through the existing pure insert/promote bridge. */
export async function appendResolvedInternalProjectArchiveV6ModelRevision(
  project: InternalProjectArchiveV6Wire,
  resolved: StandardSemModelV4AuthorityResolveResultV1,
): Promise<InternalProjectArchiveV6Wire> {
  if (resolved.readiness !== "ready" || !resolved.scientificSha256) {
    return revisionAppendFailure(
      "schema6_model_revision.ready_revision_required",
      "The native resolver did not return a ready scientific revision.",
      "Resolve the reported readiness issues before creating this revision.",
    );
  }
  const inserted = await insertInternalProjectArchiveV6ModelDraft(project, resolved.canonicalModel);
  if (inserted.status === "blocked") {
    return revisionAppendFailure(inserted.diagnostic.code, inserted.diagnostic.message, inserted.diagnostic.correctiveAction);
  }
  const record = inserted.value.project.models.find((candidate) => candidate.model_id === resolved.canonicalModel.id);
  if (record?.payload.kind !== "sem_model_v4_draft"
    || record.payload.model_document_sha256 !== resolved.modelDocumentSha256) {
    return revisionAppendFailure(
      "schema6_model_revision.document_digest_mismatch",
      "The appended draft digest differs from the native Standard resolver receipt.",
      "Keep both authorities unchanged and retry from the validated source session.",
    );
  }
  const promoted = await promoteInternalProjectArchiveV6ModelDraft(
    inserted.value.project,
    resolved.canonicalModel.id,
    resolved.modelDocumentSha256,
  );
  if (promoted.status === "blocked") {
    return revisionAppendFailure(promoted.diagnostic.code, promoted.diagnostic.message, promoted.diagnostic.correctiveAction);
  }
  const ready = promoted.value.project.models.find((candidate) => candidate.model_id === resolved.canonicalModel.id);
  if (ready?.payload.kind !== "sem_model_v4"
    || ready.payload.scientific_sha256 !== resolved.scientificSha256) {
    return revisionAppendFailure(
      "schema6_model_revision.scientific_digest_mismatch",
      "The promoted revision digest differs from the native Standard resolver receipt.",
      "Keep both authorities unchanged and retry from the validated source session.",
    );
  }
  return promoted.value.project;
}
