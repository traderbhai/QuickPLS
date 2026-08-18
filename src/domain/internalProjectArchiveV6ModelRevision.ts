import type { InternalProjectArchiveV6Wire, ProjectModelRecordV6Wire } from "./internalProjectArchiveV6Wire";
import type { SemModelV4 } from "./semModelV4";

export class InternalProjectArchiveV6ModelRevisionError extends Error {
  constructor(public readonly code: string, message: string, public readonly correctiveAction: string) {
    super(message);
    this.name = "InternalProjectArchiveV6ModelRevisionError";
  }
}

const fail = (code: string, message: string, correctiveAction: string): never => {
  throw new InternalProjectArchiveV6ModelRevisionError(code, message, correctiveAction);
};

export interface InternalProjectArchiveV6ModelRevisionV1 {
  source: ProjectModelRecordV6Wire & {
    payload: Extract<ProjectModelRecordV6Wire["payload"], { kind: "sem_model_v4" }>;
  };
  revision: SemModelV4;
}

/** Builds an unbound, new-identity revision without changing any archive artifact. */
export function buildInternalProjectArchiveV6ModelRevisionV1(
  project: InternalProjectArchiveV6Wire,
  sourceModelId: string,
  revisionModelId: string,
  revisionName: string,
): InternalProjectArchiveV6ModelRevisionV1 {
  if (!revisionModelId.trim() || revisionModelId !== revisionModelId.trim() || revisionModelId === sourceModelId) {
    return fail(
      "schema6_model_revision.new_identity_required",
      "A model revision requires a distinct, canonical nonempty model id.",
      "Create the revision again with a fresh model identity.",
    );
  }
  if (!revisionName.trim() || revisionName !== revisionName.trim()) {
    return fail(
      "schema6_model_revision.name_required",
      "A model revision requires a canonical nonempty name.",
      "Provide a revision name without surrounding whitespace.",
    );
  }
  if (project.models.some((record) => record.model_id === revisionModelId)
    || project.canonical_result_documents.some((attachment) => (
      attachment.canonical_document.provenance.model_id === revisionModelId
    ))) {
    return fail(
      "schema6_model_revision.identity_unavailable",
      `Model identity ${revisionModelId} is already reserved by this project.`,
      "Create the revision again with a fresh model identity.",
    );
  }
  const record = project.models.find((candidate) => candidate.model_id === sourceModelId);
  if (!record || record.payload.kind !== "sem_model_v4") {
    return fail(
      "schema6_model_revision.ready_source_required",
      "Only a ready SemModelV4 authority can be forked from a RecipeV4 binding.",
      "Select the clean ready model referenced by the RecipeV4.",
    );
  }
  const sourceScientificSha256 = record.payload.scientific_sha256;
  const bound = project.recipes.some((recipe) => recipe.model_binding.kind === "project_sem_model_v4_reference"
    && recipe.model_binding.model_id === sourceModelId
    && recipe.model_binding.scientific_sha256 === sourceScientificSha256);
  if (!bound) {
    return fail(
      "schema6_model_revision.recipe_binding_required",
      "The selected model is not bound by an exact RecipeV4 scientific reference.",
      "Use this action only for a clean RecipeV4-bound model.",
    );
  }
  return {
    source: record as InternalProjectArchiveV6ModelRevisionV1["source"],
    revision: { ...record.payload.model, id: revisionModelId, name: revisionName },
  };
}
