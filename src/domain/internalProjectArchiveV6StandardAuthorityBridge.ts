import type {
  InternalProjectArchiveV6Wire,
  ProjectModelRecordV6Wire,
} from "./internalProjectArchiveV6Wire";
import {
  parseStandardSemModelV4AuthorityRecordV1,
  type StandardSemModelV4AuthorityRecordV1,
} from "./standardSemModelV4Authority";
import type { StandardSemModelV4AuthorityResolveResultV1 } from "./standardSemModelV4AuthorityCas";
import {
  parseStandardSemModelV4DiagramLayoutV1,
  type StandardSemModelV4DiagramLayoutV1,
} from "./standardSemModelV4DiagramProjection";

export const INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_LAYOUT_KEY =
  "standard_sem_model_v4_diagram_layouts_v1";

export class InternalProjectArchiveV6StandardAuthorityBridgeError extends Error {
  constructor(
    public readonly code: string,
    public readonly subject: string,
    message: string,
    public readonly correctiveAction?: string,
  ) {
    super(message);
    this.name = "InternalProjectArchiveV6StandardAuthorityBridgeError";
  }
}

export interface InternalProjectArchiveV6StandardActivationV1 {
  authority: StandardSemModelV4AuthorityRecordV1;
  layout?: StandardSemModelV4DiagramLayoutV1;
  readiness: "ready" | "authoring_only";
  scientificSha256: string | null;
}

export interface InternalProjectArchiveV6StandardSaveAuthorityV1 {
  authority: StandardSemModelV4AuthorityRecordV1;
  layout: StandardSemModelV4DiagramLayoutV1;
  readiness: "ready" | "authoring_only";
  scientificSha256: string | null;
}

/** Model identities whose scientific authority is frozen by a recipe or canonical result. */
export function internalProjectArchiveV6ScientificEditLockedModelIdsV1(
  project: InternalProjectArchiveV6Wire,
): string[] {
  const activatable = new Set(project.models
    .filter((record) => record.payload.kind !== "legacy_estimand_unspecified")
    .map((record) => record.model_id));
  const locked = new Set<string>();
  project.recipes.forEach((recipe) => {
    if (recipe.model_binding.kind === "project_sem_model_v4_reference"
      && activatable.has(recipe.model_binding.model_id)) {
      locked.add(recipe.model_binding.model_id);
    }
  });
  project.canonical_result_documents.forEach((attachment) => {
    const modelId = attachment.canonical_document.provenance.model_id;
    if (activatable.has(modelId)) locked.add(modelId);
  });
  return [...locked].sort();
}

const fail = (
  code: string,
  subject: string,
  message: string,
  correctiveAction?: string,
): never => {
  throw new InternalProjectArchiveV6StandardAuthorityBridgeError(
    code,
    subject,
    message,
    correctiveAction,
  );
};

function object(value: unknown, subject: string): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return fail("schema6_standard_bridge.object_required", subject, `${subject} must be an object.`);
  }
  return value as Record<string, unknown>;
}

function sourceLayout(
  project: InternalProjectArchiveV6Wire,
  modelId: string,
): StandardSemModelV4DiagramLayoutV1 | undefined {
  const lane = project.layouts[INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_LAYOUT_KEY];
  if (lane === undefined) return undefined;
  const record = object(lane, `project.layouts.${INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_LAYOUT_KEY}`);
  if (record.schema_version !== 1) {
    return fail(
      "schema6_standard_bridge.layout_version_unsupported",
      `project.layouts.${INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_LAYOUT_KEY}.schema_version`,
      "The Standard presentation-layout lane must use schema version 1.",
    );
  }
  const models = object(
    record.models,
    `project.layouts.${INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_LAYOUT_KEY}.models`,
  );
  const candidate = models[modelId];
  return candidate === undefined ? undefined : parseStandardSemModelV4DiagramLayoutV1(candidate);
}

export function bindInternalProjectArchiveV6ModelToResolvedStandardAuthorityV1(
  record: ProjectModelRecordV6Wire,
  resolved: StandardSemModelV4AuthorityResolveResultV1,
  project: InternalProjectArchiveV6Wire,
): InternalProjectArchiveV6StandardActivationV1 {
  if (record.payload.kind === "legacy_estimand_unspecified") {
    return fail(
      "schema6_standard_bridge.legacy_model_not_activatable",
      record.model_id,
      "A legacy estimand-unspecified model cannot become a Standard SemModelV4 authority.",
    );
  }
  if (resolved.canonicalModel.id !== record.model_id) {
    return fail(
      "schema6_standard_bridge.model_identity_mismatch",
      record.model_id,
      "The native resolver returned a different model identity.",
    );
  }
  if (record.payload.kind === "sem_model_v4") {
    if (resolved.readiness !== "ready" || resolved.scientificSha256 !== record.payload.scientific_sha256) {
      return fail(
        "schema6_standard_bridge.ready_digest_mismatch",
        record.model_id,
        "The native ready scientific digest does not match the schema-6 ready-model binding.",
      );
    }
  } else if (
    resolved.modelDocumentSha256 !== record.payload.model_document_sha256
    || resolved.readiness !== "authoring_only"
    || resolved.scientificSha256 !== null
  ) {
    return fail(
      "schema6_standard_bridge.draft_digest_mismatch",
      record.model_id,
      "The native document digest or readiness does not match the schema-6 draft-model binding.",
    );
  }

  return {
    authority: parseStandardSemModelV4AuthorityRecordV1({
      schema_version: 1,
      model_document_sha256: resolved.modelDocumentSha256,
      model: resolved.canonicalModel,
    }),
    layout: sourceLayout(project, record.model_id),
    readiness: resolved.readiness,
    scientificSha256: resolved.scientificSha256,
  };
}

export function deriveInternalProjectArchiveV6StandardSaveCandidateV1(
  source: InternalProjectArchiveV6Wire,
  authorities: Readonly<Record<string, InternalProjectArchiveV6StandardSaveAuthorityV1>>,
): InternalProjectArchiveV6Wire {
  const sourceIds = source.models
    .filter((record) => record.payload.kind !== "legacy_estimand_unspecified")
    .map((record) => record.model_id);
  const suppliedIds = Object.keys(authorities);
  if (
    sourceIds.length !== suppliedIds.length
    || sourceIds.some((modelId) => !Object.prototype.hasOwnProperty.call(authorities, modelId))
  ) {
    return fail(
      "schema6_standard_bridge.authority_set_mismatch",
      "project.models",
      "The Standard authority set must exactly match every activatable schema-6 model.",
    );
  }

  source.recipes.forEach((recipe, index) => {
    const binding = recipe.model_binding;
    if (binding.kind !== "project_sem_model_v4_reference") return;
    const current = authorities[binding.model_id];
    if (current?.readiness === "ready" && current.scientificSha256 === binding.scientific_sha256) {
      return;
    }
    fail(
      "schema6_standard_bridge.recipe_model_reference_stale",
      `project.recipes[${index}].model_binding`,
      `Recipe ${recipe.id} is still bound to the previous ready scientific revision of model ${binding.model_id}; saving the current Standard authority would make that RecipeV4 reference stale.`,
      `Restore model ${binding.model_id} to the ready scientific revision referenced by recipe ${recipe.id}, or use a native validated recipe-versioning workflow before saving. QuickPLS will not rewrite or discard the recipe binding.`,
    );
  });

  const models = source.models.map((record): ProjectModelRecordV6Wire => {
    if (record.payload.kind === "legacy_estimand_unspecified") return record;
    const current = authorities[record.model_id];
    if (current.authority.model.id !== record.model_id || current.layout.model_id !== record.model_id) {
      return fail(
        "schema6_standard_bridge.model_identity_mismatch",
        record.model_id,
        "A Standard authority or presentation layout belongs to a different model.",
      );
    }
    if (current.readiness === "ready") {
      if (current.scientificSha256 === null) {
        return fail(
          "schema6_standard_bridge.ready_digest_missing",
          record.model_id,
          "A ready Standard authority requires its native scientific digest.",
        );
      }
      return {
        model_id: record.model_id,
        payload: {
          kind: "sem_model_v4",
          model: current.authority.model,
          scientific_sha256: current.scientificSha256,
        },
      };
    }
    if (current.scientificSha256 !== null) {
      return fail(
        "schema6_standard_bridge.draft_digest_forbidden",
        record.model_id,
        "An authoring-only Standard authority cannot carry a scientific digest.",
      );
    }
    return {
      model_id: record.model_id,
      payload: {
        kind: "sem_model_v4_draft",
        model: current.authority.model,
        model_document_sha256: current.authority.model_document_sha256,
      },
    };
  });

  return {
    ...source,
    models,
    layouts: {
      ...source.layouts,
      [INTERNAL_PROJECT_ARCHIVE_V6_STANDARD_LAYOUT_KEY]: {
        schema_version: 1,
        models: Object.fromEntries(sourceIds.map((modelId) => [modelId, authorities[modelId].layout])),
      },
    },
  };
}
