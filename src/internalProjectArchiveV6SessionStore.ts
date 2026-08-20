import { create } from "zustand";
import type {
  InternalProjectArchiveV6ReadDiagnosticV1,
  InternalProjectArchiveV6ReadOutcomeV1,
  InternalProjectArchiveV6ReadSnapshotV1,
} from "./domain/internalProjectArchiveV6Read";
import type {
  InternalProjectArchiveV6ModelMutationDiagnosticV1,
  InternalProjectArchiveV6ModelMutationOutcomeV1,
  InternalProjectArchiveV6ModelMutationV1,
} from "./domain/internalProjectArchiveV6ModelMutation";
import type {
  InternalProjectArchiveV6Wire,
  ProjectModelRecordV6Wire,
  ProjectModelPayloadV6Wire,
} from "./domain/internalProjectArchiveV6Wire";
import { supportsGeneralSemV1 } from "./domain/internalProjectArchiveV6Wire";
import type { SemModelV4 } from "./domain/semModelV4";
import { buildInternalProjectArchiveV6ModelRevisionV1 } from "./domain/internalProjectArchiveV6ModelRevision";
import type {
  InternalProjectArchiveV6SaveCopyDiagnosticV1,
  InternalProjectArchiveV6SaveCopyOutcomeV1,
} from "./domain/internalProjectArchiveV6SaveCopy";
import {
  bindInternalProjectArchiveV6ModelToResolvedStandardAuthorityV1,
  deriveInternalProjectArchiveV6StandardSaveCandidateV1,
  internalProjectArchiveV6ScientificEditLockedModelIdsV1,
} from "./domain/internalProjectArchiveV6StandardAuthorityBridge";
import type {
  StandardSemModelV4AuthorityCasDiagnosticV1,
  StandardSemModelV4AuthorityResolveOutcomeV1,
} from "./domain/standardSemModelV4AuthorityCas";
import { saveInternalProjectArchiveV6Copy } from "./services/internalProjectArchiveV6SaveCopyService";
import { appendResolvedInternalProjectArchiveV6ModelRevision } from "./services/internalProjectArchiveV6ModelMutationService";
import { invalidateNativeGeneralSemFreshDraftAuthorityV1 } from "./services/projectService";
import { resolveStandardSemModelV4Authority } from "./services/standardSemModelV4AuthorityService";
import { useWorkspace } from "./store";
import { parseStandardSemModelV4AuthorityRecordV1 } from "./domain/standardSemModelV4Authority";
import type { StandardSemModelV4AuthorityResolveResultV1 } from "./domain/standardSemModelV4AuthorityCas";
import {
  reduceStandardSemModelV4AuthorityV1,
} from "./domain/standardSemModelV4Authority";
import type {
  GeneralSemExecutionAuthorityRevisionIdentityV1,
  GeneralSemExecutionAuthorityRevisionEditorIntentV1,
  GeneralSemExecutionAuthoritySourcePinV1,
  InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1,
} from "./domain/internalGeneralSemExecutionAuthorityRevisionV1";
import {
  saveGeneralSemExecutionAuthorityRevisionV1,
  type InternalGeneralSemExecutionAuthorityRevisionPersistOutcomeV1,
} from "./services/internalGeneralSemExecutionAuthorityRevisionService";

export const INTERNAL_PROJECT_ARCHIVE_V6_SESSION_CAPABILITIES = Object.freeze({
  edit: false,
  ephemeralModelAuthorityMutation: false,
  compile: false,
  run: false,
  save: false,
  saveAs: false,
  saveCopy: "new_destination_only",
  autosave: false,
  recovery: false,
} as const);

export interface InternalProjectArchiveV6ReadOnlySession {
  kind: "internal_schema6_read_only";
  access: "read_only";
  /** Immutable strict-reader receipt for the source archive. */
  snapshot: InternalProjectArchiveV6ReadSnapshotV1;
  /** First strict-reader receipt; never changes when a saved copy is reanchored. */
  originSnapshot?: InternalProjectArchiveV6ReadSnapshotV1;
  /** Current strictly validated save-copy anchor; never mutated in place. */
  project: InternalProjectArchiveV6Wire;
  standardActivation?: {
    modelIds: string[];
    sourceArchiveSha256: string;
  } | null;
  capabilities: typeof INTERNAL_PROJECT_ARCHIVE_V6_SESSION_CAPABILITIES;
}

export type InternalProjectArchiveV6SessionPhase =
  | "inactive"
  | "opening"
  | "active"
  | "error";

export type InternalProjectArchiveV6SessionOpenResult =
  | "activated"
  | "blocked"
  | "cancelled"
  | "stale";

export type InternalProjectArchiveV6SessionLoader = () => Promise<
  InternalProjectArchiveV6ReadOutcomeV1 | null
>;

export type InternalProjectArchiveV6ModelMutationExecutor = (
  project: InternalProjectArchiveV6Wire,
  mutation: InternalProjectArchiveV6ModelMutationV1,
) => Promise<InternalProjectArchiveV6ModelMutationOutcomeV1>;

export type InternalProjectArchiveV6ModelMutationApplyResult =
  | "applied"
  | "blocked"
  | "inactive"
  | "stale";

export type InternalProjectArchiveV6StandardAuthorityResolver = (
  model: SemModelV4,
) => Promise<StandardSemModelV4AuthorityResolveOutcomeV1>;

export type InternalProjectArchiveV6GeneralSemDraftRevoker = () => Promise<void>;

export type InternalProjectArchiveV6ModelRevisionAppender = (
  project: InternalProjectArchiveV6Wire,
  resolved: StandardSemModelV4AuthorityResolveResultV1,
) => Promise<InternalProjectArchiveV6Wire>;

export interface InternalProjectArchiveV6ModelRevisionForkOptions {
  revisionModelId?: string;
  revisionName?: string;
  resolver?: InternalProjectArchiveV6StandardAuthorityResolver;
  appender?: InternalProjectArchiveV6ModelRevisionAppender;
}

export type InternalProjectArchiveV6StandardActivationApplyResult =
  | "activated"
  | "blocked"
  | "inactive"
  | "stale";

export type InternalProjectArchiveV6SaveCopyExecutor = (
  snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  project: InternalProjectArchiveV6Wire,
) => Promise<InternalProjectArchiveV6SaveCopyOutcomeV1 | null>;

export type InternalProjectArchiveV6SaveCopyApplyResult =
  | "saved"
  | "blocked"
  | "cancelled"
  | "inactive"
  | "stale";

export type InternalProjectArchiveV6CloseStandardProjectResult =
  | "closed"
  | "blocked"
  | "inactive";

export type InternalProjectArchiveV6GeneralSemReanchorResult =
  | "reanchored"
  | "blocked"
  | "inactive";

export type InternalProjectArchiveV6GeneralSemRevisionResult =
  | "saved"
  | "blocked"
  | "cancelled"
  | "inactive"
  | "stale";

export interface InternalProjectArchiveV6GeneralSemRevisionExecutorInputV1 {
  snapshot: InternalProjectArchiveV6ReadSnapshotV1;
  source: GeneralSemExecutionAuthoritySourcePinV1;
  revision: GeneralSemExecutionAuthorityRevisionIdentityV1;
  intent: GeneralSemExecutionAuthorityRevisionEditorIntentV1;
  revisionNumberHint: number;
  experimentalLabsEnabled: boolean;
}

export type InternalProjectArchiveV6GeneralSemRevisionExecutorV1 = (
  input: InternalProjectArchiveV6GeneralSemRevisionExecutorInputV1,
) => Promise<InternalGeneralSemExecutionAuthorityRevisionPersistOutcomeV1 | null>;

export interface InternalProjectArchiveV6GeneralSemRevisionOptionsV1 {
  intent: GeneralSemExecutionAuthorityRevisionEditorIntentV1;
  projectId?: string;
  projectName?: string;
  modelId?: string;
  modelName?: string;
  recipeId?: string;
  createdAt?: string;
  executor?: InternalProjectArchiveV6GeneralSemRevisionExecutorV1;
  resolver?: InternalProjectArchiveV6StandardAuthorityResolver;
}

export interface InternalProjectArchiveV6SessionState {
  phase: InternalProjectArchiveV6SessionPhase;
  session: InternalProjectArchiveV6ReadOnlySession | null;
  failure: InternalProjectArchiveV6ReadDiagnosticV1 | null;
  statusMessage: string;
  requestEpoch: number;
  dirty: boolean;
  persistence: "not_persisted" | "persisted_new_copy" | "persisted_validated_archive" | null;
  modelMutationPending: boolean;
  modelMutationFailure: InternalProjectArchiveV6ModelMutationDiagnosticV1 | null;
  modelMutationStatusMessage: string;
  standardActivationPending: boolean;
  standardActivationFailure: StandardSemModelV4AuthorityCasDiagnosticV1 | null;
  standardActivationStatusMessage: string;
  revisionForkPending: boolean;
  revisionForkFailure: StandardSemModelV4AuthorityCasDiagnosticV1 | null;
  revisionForkStatusMessage: string;
  saveCopyPending: boolean;
  saveCopyFailure: InternalProjectArchiveV6SaveCopyDiagnosticV1 | null;
  saveCopyStatusMessage: string;
  open: (
    loader: InternalProjectArchiveV6SessionLoader,
  ) => Promise<InternalProjectArchiveV6SessionOpenResult>;
  mutateModel: (
    mutation: InternalProjectArchiveV6ModelMutationV1,
    executor?: InternalProjectArchiveV6ModelMutationExecutor,
  ) => Promise<InternalProjectArchiveV6ModelMutationApplyResult>;
  activateStandardAuthorities: (
    resolver?: InternalProjectArchiveV6StandardAuthorityResolver,
    revokeGeneralSemDraft?: InternalProjectArchiveV6GeneralSemDraftRevoker,
  ) => Promise<InternalProjectArchiveV6StandardActivationApplyResult>;
  forkActiveRecipeBoundRevision: (
    options?: InternalProjectArchiveV6ModelRevisionForkOptions,
  ) => Promise<InternalProjectArchiveV6StandardActivationApplyResult>;
  saveCopy: (
    executor?: InternalProjectArchiveV6SaveCopyExecutor,
  ) => Promise<InternalProjectArchiveV6SaveCopyApplyResult>;
  reviseGeneralSemExecutionAuthority: (
    options: InternalProjectArchiveV6GeneralSemRevisionOptionsV1,
  ) => Promise<InternalProjectArchiveV6GeneralSemRevisionResult>;
  reanchorGeneralSemSnapshot: (
    snapshot: InternalProjectArchiveV6ReadSnapshotV1,
  ) => InternalProjectArchiveV6GeneralSemReanchorResult;
  closeStandardProject: () => InternalProjectArchiveV6CloseStandardProjectResult;
  deactivate: () => void;
}

function failureFromUnknown(
  error: unknown,
): InternalProjectArchiveV6ReadDiagnosticV1 {
  const record = error && typeof error === "object"
    ? error as Record<string, unknown>
    : null;
  return {
    code: record && typeof record.code === "string"
      ? record.code
      : "schema6_archive_session.open_failed",
    message: error instanceof Error && error.message
      ? error.message
      : "QuickPLS could not open the read-only schema-6 session.",
    correctiveAction: record && typeof record.correctiveAction === "string"
      ? record.correctiveAction
      : "Confirm the archive is a trusted schema-6 .qpls ZIP, then open it again.",
  };
}

function stablePhase(
  session: InternalProjectArchiveV6ReadOnlySession | null,
): "active" | "inactive" {
  return session ? "active" : "inactive";
}

function saveCopyFailureFromUnknown(
  error: unknown,
): InternalProjectArchiveV6SaveCopyDiagnosticV1 {
  const record = error && typeof error === "object"
    ? error as Record<string, unknown>
    : null;
  return {
    code: record && typeof record.code === "string"
      ? record.code
      : "schema6_save_copy.session_failed",
    message: error instanceof Error && error.message
      ? error.message
      : "QuickPLS could not save the detached schema-6 document to a new copy.",
    correctiveAction: record && typeof record.correctiveAction === "string"
      ? record.correctiveAction
      : "Keep this Labs session open and retry with a new local .qpls destination.",
  };
}

function standardActivationFailureFromUnknown(
  error: unknown,
): StandardSemModelV4AuthorityCasDiagnosticV1 {
  const record = error && typeof error === "object"
    ? error as Record<string, unknown>
    : null;
  return {
    code: record && typeof record.code === "string"
      ? record.code
      : "schema6_standard_activation.failed",
    message: error instanceof Error && error.message
      ? error.message
      : "QuickPLS could not activate the schema-6 models as Standard authorities.",
    correctiveAction: record && typeof record.correctiveAction === "string"
      ? record.correctiveAction
      : "Keep the source archive open, resolve the reported model issue, and activate again.",
    authoringIssues: [],
    readinessIssues: [],
  };
}

function revisionFailure(
  diagnostic: InternalGeneralSemExecutionAuthorityRevisionDiagnosticV1,
): StandardSemModelV4AuthorityCasDiagnosticV1 {
  return { ...diagnostic, authoringIssues: [], readinessIssues: [] };
}

function revisionForkFailureFromUnknown(error: unknown): StandardSemModelV4AuthorityCasDiagnosticV1 {
  const record = error && typeof error === "object" ? error as Record<string, unknown> : null;
  return {
    code: record && typeof record.code === "string" ? record.code : "schema6_model_revision.failed",
    message: error instanceof Error && error.message ? error.message : "QuickPLS could not create the model revision.",
    correctiveAction: record && typeof record.correctiveAction === "string"
      ? record.correctiveAction
      : "Keep the original authority unchanged and retry from the clean RecipeV4-bound model.",
    authoringIssues: [],
    readinessIssues: [],
  };
}

/**
 * Internal/Labs source-anchor boundary for strict schema-6 activation.
 *
 * Opening is isolated. Only the explicit, native-resolved activation action
 * can install Standard authorities. The first source snapshot is immutable;
 * successful new-destination copies move only the current validated anchor.
 * Request epochs prevent late native responses from changing either store.
 */
export const useInternalProjectArchiveV6Session =
  create<InternalProjectArchiveV6SessionState>((set, get) => ({
    phase: "inactive",
    session: null,
    failure: null,
    statusMessage: "No schema-6 read-only session is active.",
    requestEpoch: 0,
    dirty: false,
    persistence: null,
    modelMutationPending: false,
    modelMutationFailure: null,
    modelMutationStatusMessage: "Detached schema-6 model mutation is disabled; activate Standard authority instead.",
    standardActivationPending: false,
    standardActivationFailure: null,
    standardActivationStatusMessage: "Schema-6 models have not been activated in Standard.",
    revisionForkPending: false,
    revisionForkFailure: null,
    revisionForkStatusMessage: "No RecipeV4-bound model revision has been created.",
    saveCopyPending: false,
    saveCopyFailure: null,
    saveCopyStatusMessage: "No schema-6 copy has been saved from this session.",

    open: async (loader) => {
      if (get().session?.standardActivation) {
        set({
          statusMessage: "Open blocked because the active Standard authority is still bound to this schema-6 source session.",
          standardActivationStatusMessage: "Keep the bound source open and save through its validated new-copy workflow.",
        });
        return "blocked";
      }
      const requestEpoch = get().requestEpoch + 1;
      set({
        phase: "opening",
        failure: null,
        statusMessage: "Opening the archive through the strict schema-6 ZIP reader…",
        requestEpoch,
        modelMutationPending: false,
        modelMutationFailure: null,
        standardActivationPending: false,
        standardActivationFailure: null,
        revisionForkPending: false,
        revisionForkFailure: null,
        saveCopyPending: false,
        saveCopyFailure: null,
      });

      try {
        const outcome = await loader();
        if (get().requestEpoch !== requestEpoch) return "stale";

        if (outcome === null) {
          const session = get().session;
          set({
            phase: stablePhase(session),
            failure: null,
            statusMessage: session
              ? "Archive selection cancelled. The existing read-only session remains active."
              : "Archive selection cancelled. No schema-6 session was opened.",
          });
          return "cancelled";
        }

        if (outcome.status === "blocked") {
          const session = get().session;
          set({
            phase: session ? "active" : "error",
            failure: outcome.diagnostic,
            statusMessage: session
              ? "The new archive was blocked. The existing read-only session remains active."
              : "The archive was blocked and no schema-6 session was opened.",
          });
          return "blocked";
        }

        set({
          phase: "active",
          session: {
            kind: "internal_schema6_read_only",
            access: "read_only",
            snapshot: outcome.value,
            originSnapshot: outcome.value,
            project: outcome.value.project,
            standardActivation: null,
            capabilities: INTERNAL_PROJECT_ARCHIVE_V6_SESSION_CAPABILITIES,
          },
          failure: null,
          statusMessage: "Schema-6 archive opened in the isolated read-only Labs memory session.",
          dirty: false,
          persistence: null,
          modelMutationPending: false,
          modelMutationFailure: null,
          modelMutationStatusMessage: "Detached schema-6 model mutation is disabled; activate Standard authority instead.",
          standardActivationPending: false,
          standardActivationFailure: null,
          standardActivationStatusMessage: "Schema-6 models are loaded but not yet activated in Standard.",
          revisionForkPending: false,
          revisionForkFailure: null,
          revisionForkStatusMessage: "No RecipeV4-bound model revision has been created.",
          saveCopyPending: false,
          saveCopyFailure: null,
          saveCopyStatusMessage: "No schema-6 copy has been saved from this session.",
        });
        return "activated";
      } catch (error) {
        if (get().requestEpoch !== requestEpoch) return "stale";
        const session = get().session;
        set({
          phase: session ? "active" : "error",
          failure: failureFromUnknown(error),
          statusMessage: session
            ? "The new archive could not be opened. The existing read-only session remains active."
            : "The archive could not be opened and no schema-6 session is active.",
        });
        return "blocked";
      }
    },

    mutateModel: async (_mutation, _executor) => {
      const session = get().session;
      if (!session) {
        set({
          modelMutationFailure: {
            code: "schema6_model_mutation.session_required",
            message: "No isolated schema-6 session is active.",
            correctiveAction: "Open a schema-6 archive in the Labs memory session first.",
          },
          modelMutationStatusMessage: "Model change blocked because no read-only session is active.",
        });
        return "inactive";
      }
      set({
        modelMutationPending: false,
        modelMutationFailure: {
          code: "schema6_model_mutation.standard_authority_required",
          message: "Detached schema-6 model mutation is disabled because it would create a second scientific authority.",
          correctiveAction: "Activate the loaded ready/draft models in Standard, then edit the strict Standard authority.",
        },
        modelMutationStatusMessage: "Detached mutation blocked; Standard SemModelV4 is the only mutable science authority.",
      });
      return "blocked";
    },

    activateStandardAuthorities: async (
      resolver = resolveStandardSemModelV4Authority,
      revokeGeneralSemDraft = invalidateNativeGeneralSemFreshDraftAuthorityV1,
    ) => {
      const session = get().session;
      if (!session) {
        set({
          standardActivationFailure: {
            code: "schema6_standard_activation.session_required",
            message: "No strict schema-6 session is active.",
            correctiveAction: "Open a schema-6 archive in Experimental Labs first.",
            authoringIssues: [],
            readinessIssues: [],
          },
          standardActivationStatusMessage: "Standard activation was blocked because no schema-6 session is active.",
        });
        return "inactive";
      }
      if (get().standardActivationPending || get().saveCopyPending) return "blocked";

      const records = session.project.models.filter((record): record is ProjectModelRecordV6Wire & {
        payload: Extract<ProjectModelPayloadV6Wire, { kind: "sem_model_v4" | "sem_model_v4_draft" }>;
      } => record.payload.kind === "sem_model_v4" || record.payload.kind === "sem_model_v4_draft");
      if (!records.length) {
        set({
          standardActivationFailure: {
            code: "schema6_standard_activation.no_activatable_models",
            message: "The schema-6 archive contains no ready or draft SemModelV4 model.",
            correctiveAction: "Upgrade or author a SemModelV4 model before activating Standard authority.",
            authoringIssues: [],
            readinessIssues: [],
          },
          standardActivationStatusMessage: "No schema-6 SemModelV4 authority was available to activate.",
        });
        return "blocked";
      }

      const requestEpoch = get().requestEpoch + 1;
      const workspaceBefore = useWorkspace.getState();
      set({
        requestEpoch,
        standardActivationPending: true,
        standardActivationFailure: null,
        standardActivationStatusMessage: "Resolving native Standard model and scientific digests…",
      });

      try {
        if (supportsGeneralSemV1(session.project)) {
          await revokeGeneralSemDraft();
          if (get().requestEpoch !== requestEpoch || get().session !== session) return "stale";
        }
        const outcomes = await Promise.all(records.map((record) => resolver(record.payload.model)));
        if (get().requestEpoch !== requestEpoch) return "stale";
        if (useWorkspace.getState() !== workspaceBefore) {
          set({
            standardActivationPending: false,
            standardActivationStatusMessage: "Standard activation became stale because the workspace changed; nothing was installed.",
          });
          return "stale";
        }
        const blocked = outcomes.find((outcome) => outcome.status === "blocked");
        if (blocked?.status === "blocked") {
          set({
            standardActivationPending: false,
            standardActivationFailure: blocked.diagnostic,
            standardActivationStatusMessage: "Native authority resolution blocked activation; both stores are unchanged.",
          });
          return "blocked";
        }
        const installations = outcomes.map((outcome, index) => {
          if (outcome.status !== "ok") throw new Error("Native authority resolution did not return an accepted model.");
          return bindInternalProjectArchiveV6ModelToResolvedStandardAuthorityV1(
            records[index],
            outcome.value,
            session.project,
          );
        });
        if (get().requestEpoch !== requestEpoch || get().session !== session) return "stale";
        const installed = useWorkspace.getState().activateStandardSemModelV4Authorities(
          installations,
          records[0].model_id,
          session.project.name,
          session.project.datasets.map((dataset) => ({
            id: dataset.id,
            name: dataset.name,
            columns: dataset.schema.columns.map((column) => column.name),
            columnMetadata: dataset.schema.columns.map((column) => ({ ...column })),
            rowCount: dataset.schema.case_count,
            fingerprint: dataset.fingerprint,
            kind: dataset.schema.kind,
            sampleSize: dataset.schema.sample_size,
          })),
          internalProjectArchiveV6ScientificEditLockedModelIdsV1(session.project),
        );
        if (!installed) throw new Error("The resolved Standard authority set could not be installed atomically.");
        if (supportsGeneralSemV1(session.project)) {
          useWorkspace.getState().setProjectWritable(false);
        }
        const current = get().session;
        if (!current || current !== session || get().requestEpoch !== requestEpoch) return "stale";
        set({
          session: {
            ...current,
            standardActivation: {
              modelIds: records.map((record) => record.model_id),
              sourceArchiveSha256: session.snapshot.archiveSha256,
            },
          },
          dirty: false,
          persistence: supportsGeneralSemV1(session.project) ? "persisted_validated_archive" : null,
          standardActivationPending: false,
          standardActivationFailure: null,
          standardActivationStatusMessage: `${records.length} schema-6 model${records.length === 1 ? "" : "s"} activated as the sole Standard science authority.`,
        });
        return "activated";
      } catch (error) {
        if (get().requestEpoch !== requestEpoch) return "stale";
        set({
          standardActivationPending: false,
          standardActivationFailure: standardActivationFailureFromUnknown(error),
          standardActivationStatusMessage: "Standard activation failed closed; both stores are unchanged.",
        });
        return "blocked";
      }
    },

    reviseGeneralSemExecutionAuthority: async (options) => {
      const session = get().session;
      const activation = session?.standardActivation;
      const workspace = useWorkspace.getState();
      const activeModelId = workspace.activeModelId;
      const authority = activeModelId
        ? workspace.standardSemModelV4Authorities[activeModelId]
        : null;
      const nativeAuthority = session?.snapshot.generalSemExecutionAuthority;
      if (!session || !activation || !supportsGeneralSemV1(session.project)
        || !activeModelId || !authority || !nativeAuthority) {
        set({
          revisionForkFailure: {
            code: "schema6_general_sem_revision.active_authority_required",
            message: "An activated, strictly reopened general_sem_v1 model and RecipeV4 authority is required.",
            correctiveAction: "Open and activate the exact marked General SEM archive before creating a revision.",
            authoringIssues: [], readinessIssues: [],
          },
          revisionForkStatusMessage: "General SEM revision blocked because no exact marked authority is active.",
        });
        return "inactive";
      }
      if (get().standardActivationPending || get().revisionForkPending || get().saveCopyPending) {
        set({
          revisionForkFailure: {
            code: "schema6_general_sem_revision.operation_pending",
            message: "Another schema-6 authority operation is still running.",
            correctiveAction: "Wait for the current operation to finish, then create the revision again.",
            authoringIssues: [], readinessIssues: [],
          },
          revisionForkStatusMessage: "General SEM revision was not started while another operation was pending.",
        });
        return "blocked";
      }
      const sourceRecord = session.project.models.find((record) => record.model_id === activeModelId);
      const sourceRecipe = session.project.recipes.find((recipe) => recipe.id === nativeAuthority.recipeId);
      const captured = workspace.captureStandardSemModelV4SaveAuthorities(activation.modelIds);
      const persistence = workspace.standardSemModelV4Persistence[activeModelId];
      if (activation.modelIds.length !== 1
        || sourceRecord?.payload.kind !== "sem_model_v4"
        || !sourceRecipe
        || nativeAuthority.projectId !== session.project.project_id
        || nativeAuthority.modelId !== activeModelId
        || nativeAuthority.modelScientificSha256 !== sourceRecord.payload.scientific_sha256
        || persistence?.scientificSha256 !== nativeAuthority.modelScientificSha256
        || !captured
        || Object.values(captured).some((entry) => entry.dirty)
        || get().dirty) {
        set({
          revisionForkFailure: {
            code: "schema6_general_sem_revision.clean_source_required",
            message: "The active canvas, source model, RecipeV4, or archive digest is stale or unsaved.",
            correctiveAction: "Restore the exact clean strictly reopened General SEM authority, then retry.",
            authoringIssues: [], readinessIssues: [],
          },
          revisionForkStatusMessage: "General SEM revision blocked before native file selection; the source is unchanged.",
        });
        return "blocked";
      }

      try {
        const preview = reduceStandardSemModelV4AuthorityV1(authority, options.intent);
        if (preview.readiness !== "ready") {
          set({
            revisionForkFailure: {
              code: "schema6_general_sem_revision.revised_model_not_ready",
              message: "The atomic scientific edit would not produce a calculation-ready SemModelV4.",
              correctiveAction: "Resolve the reported model readiness issues before saving a General SEM revision.",
              authoringIssues: [],
              readinessIssues: [...preview.readiness_issues],
            },
            revisionForkStatusMessage: "General SEM revision blocked by the strict authority preview.",
          });
          return "blocked";
        }
      } catch (error) {
        set({
          revisionForkFailure: revisionForkFailureFromUnknown(error),
          revisionForkStatusMessage: "General SEM revision intent was rejected atomically; the source is unchanged.",
        });
        return "blocked";
      }

      const lineage = session.project.layouts.general_sem_execution_authority_revision_v1 as Record<string, unknown> | undefined;
      const currentRevision = Number.isSafeInteger(lineage?.revisionNumber) && (lineage?.revisionNumber as number) > 0
        ? lineage?.revisionNumber as number
        : 0;
      const revisionNumberHint = currentRevision + 1;
      const revision: GeneralSemExecutionAuthorityRevisionIdentityV1 = {
        projectId: options.projectId ?? globalThis.crypto.randomUUID(),
        projectName: options.projectName ?? `${session.project.name} — SEM revision ${revisionNumberHint}`,
        createdAt: options.createdAt ?? new Date().toISOString(),
        modelId: options.modelId ?? `model:general-sem-revision:${globalThis.crypto.randomUUID()}`,
        modelName: options.modelName ?? `${authority.model.name} revision ${revisionNumberHint}`,
        recipeId: options.recipeId ?? globalThis.crypto.randomUUID(),
      };
      const source: GeneralSemExecutionAuthoritySourcePinV1 = {
        projectId: nativeAuthority.projectId,
        modelId: activeModelId,
        modelDocumentSha256: authority.model_document_sha256,
        modelScientificSha256: nativeAuthority.modelScientificSha256,
        recipeId: nativeAuthority.recipeId,
        recipeDocumentSha256: nativeAuthority.recipeDocumentSha256,
      };
      const sourceWorkspaceMeta = {
        projectName: workspace.projectName,
        projectPath: workspace.projectPath,
        projectId: workspace.projectId,
        descriptors: Object.values(workspace.standardSemModelV4DatasetDescriptors),
        lockedIds: Object.keys(workspace.standardSemModelV4ScientificEditLocks),
      };
      const requestEpoch = get().requestEpoch + 1;
      set({
        requestEpoch,
        revisionForkPending: true,
        revisionForkFailure: null,
        revisionForkStatusMessage: "Choose a new .qpls destination for the versioned General SEM model-and-Recipe revision…",
      });

      let outcome: InternalGeneralSemExecutionAuthorityRevisionPersistOutcomeV1 | null;
      try {
        outcome = await (options.executor ?? saveGeneralSemExecutionAuthorityRevisionV1)({
          snapshot: session.snapshot,
          source,
          revision,
          intent: options.intent,
          revisionNumberHint,
          experimentalLabsEnabled: workspace.uiPreferences.experimentalLabsEnabled,
        });
      } catch (error) {
        if (get().requestEpoch !== requestEpoch) return "stale";
        set({
          revisionForkPending: false,
          revisionForkFailure: revisionForkFailureFromUnknown(error),
          revisionForkStatusMessage: "General SEM revision failed before activation; the source archive and workspace are unchanged.",
        });
        return "blocked";
      }
      if (get().requestEpoch !== requestEpoch || get().session !== session) return "stale";
      if (outcome === null) {
        set({
          revisionForkPending: false,
          revisionForkFailure: null,
          revisionForkStatusMessage: "General SEM revision cancelled before a native write; the source is unchanged.",
        });
        return "cancelled";
      }
      if (outcome.status === "blocked") {
        set({
          revisionForkPending: false,
          revisionForkFailure: revisionFailure(outcome.diagnostic),
          revisionForkStatusMessage: outcome.persistedReceipt
            ? `A new revision was persisted at ${outcome.persistedReceipt.destinationArchivePath}, but activation was blocked; the source remains active.`
            : "General SEM revision was blocked; the source archive and workspace are unchanged.",
        });
        return "blocked";
      }
      const { receipt, snapshot: revisedSnapshot } = outcome.value;
      if (receipt.sourceArchiveBytes !== session.snapshot.archiveBytes
        || receipt.revisionNumber !== revisionNumberHint) {
        set({
          revisionForkPending: false,
          revisionForkFailure: revisionFailure({
            code: "schema6_general_sem_revision.source_or_version_mismatch",
            message: "The persisted revision receipt differs from the active source bytes or expected one-step revision number.",
            correctiveAction: `Keep the current source active and inspect ${receipt.destinationArchivePath} separately.`,
          }),
          revisionForkStatusMessage: `Revision ${receipt.destinationArchivePath} was persisted but not activated; the source remains active.`,
        });
        return "blocked";
      }

      const currentCaptured = useWorkspace.getState().captureStandardSemModelV4SaveAuthorities(activation.modelIds);
      if (!currentCaptured
        || Object.values(currentCaptured).some((entry) => entry.dirty)
        || JSON.stringify(currentCaptured) !== JSON.stringify(captured)
        || get().dirty) {
        set({
          revisionForkPending: false,
          revisionForkFailure: null,
          revisionForkStatusMessage: `Revision ${receipt.destinationArchivePath} was persisted, but newer canvas changes made activation stale; the source remains active.`,
        });
        return "stale";
      }
      const revisedRecord = revisedSnapshot.project.models.find((record) => record.model_id === receipt.residentModelId);
      if (revisedRecord?.payload.kind !== "sem_model_v4") {
        set({
          revisionForkPending: false,
          revisionForkFailure: revisionFailure({
            code: "schema6_general_sem_revision.revised_model_missing",
            message: "The strict destination snapshot does not expose the promoted revised SemModelV4.",
            correctiveAction: `Keep the source active and inspect ${receipt.destinationArchivePath} separately.`,
          }),
          revisionForkStatusMessage: "Persisted General SEM revision was not activated; the source remains active.",
        });
        return "blocked";
      }
      let resolved: Awaited<ReturnType<typeof resolveStandardSemModelV4Authority>>;
      try {
        resolved = await (options.resolver ?? resolveStandardSemModelV4Authority)(revisedRecord.payload.model);
      } catch (error) {
        if (get().requestEpoch !== requestEpoch || get().session !== session) return "stale";
        const cause = revisionForkFailureFromUnknown(error);
        set({
          revisionForkPending: false,
          revisionForkFailure: revisionFailure({
            code: "schema6_general_sem_revision.native_resolution_failed_after_persist",
            message: `The revision was persisted, but native authority resolution failed: ${cause.message}`,
            correctiveAction: `The unchanged source remains active. The revision is saved at ${receipt.destinationArchivePath}; inspect and reopen it explicitly before using it.`,
          }),
          revisionForkStatusMessage: `Revision ${receipt.destinationArchivePath} was persisted but not activated after native resolution failed; the source remains active.`,
        });
        return "blocked";
      }
      if (get().requestEpoch !== requestEpoch || get().session !== session) return "stale";
      if (resolved.status === "blocked"
        || resolved.value.modelDocumentSha256 !== receipt.residentModelDocumentSha256
        || resolved.value.scientificSha256 !== receipt.residentModelScientificSha256) {
        const diagnostic = resolved.status === "blocked"
          ? resolved.diagnostic
          : revisionFailure({
            code: "schema6_general_sem_revision.native_resolution_mismatch",
            message: "Native Standard resolution differs from the persisted revision receipt.",
            correctiveAction: `Keep the source active and inspect ${receipt.destinationArchivePath} separately.`,
          });
        set({
          revisionForkPending: false,
          revisionForkFailure: diagnostic,
          revisionForkStatusMessage: `Revision ${receipt.destinationArchivePath} was persisted but failed closed before workspace replacement; the source remains active.`,
        });
        return "blocked";
      }
      const installation = bindInternalProjectArchiveV6ModelToResolvedStandardAuthorityV1(
        revisedRecord,
        resolved.value,
        revisedSnapshot.project,
      );
      const revisedDescriptors = revisedSnapshot.project.datasets.map((dataset) => ({
        id: dataset.id,
        name: dataset.name,
        columns: dataset.schema.columns.map((column) => column.name),
        columnMetadata: dataset.schema.columns.map((column) => ({ ...column })),
        rowCount: dataset.schema.case_count,
        fingerprint: dataset.fingerprint,
        kind: dataset.schema.kind,
        sampleSize: dataset.schema.sample_size,
      }));

      const replacingWorkspace = useWorkspace.getState();
      if (!replacingWorkspace.clearStandardSemModelV4Workspace(activation.modelIds)) {
        set({
          revisionForkPending: false,
          revisionForkFailure: revisionFailure({
            code: "schema6_general_sem_revision.source_workspace_stale",
            message: "The source workspace could not be released atomically for revision activation.",
            correctiveAction: `The revision is saved at ${receipt.destinationArchivePath}; keep the source open and activate it later.`,
          }),
          revisionForkStatusMessage: `Revision ${receipt.destinationArchivePath} was persisted but not activated; the source remains active.`,
        });
        return "stale";
      }
      const installed = useWorkspace.getState().activateStandardSemModelV4Authorities(
        [installation],
        receipt.residentModelId,
        revisedSnapshot.project.name,
        revisedDescriptors,
        internalProjectArchiveV6ScientificEditLockedModelIdsV1(revisedSnapshot.project),
      );
      if (!installed) {
        const restored = useWorkspace.getState().activateStandardSemModelV4Authorities(
          Object.values(captured),
          activeModelId,
          sourceWorkspaceMeta.projectName,
          sourceWorkspaceMeta.descriptors,
          sourceWorkspaceMeta.lockedIds,
        );
        if (restored) useWorkspace.getState().setProjectMeta(
          sourceWorkspaceMeta.projectName,
          sourceWorkspaceMeta.projectPath,
          sourceWorkspaceMeta.projectId,
        );
        set({
          revisionForkPending: false,
          revisionForkFailure: revisionFailure({
            code: restored
              ? "schema6_general_sem_revision.activation_failed_source_restored"
              : "schema6_general_sem_revision.activation_failed_recovery_required",
            message: restored
              ? "The revised workspace could not be installed, so QuickPLS restored the exact source authority."
              : "The revised workspace could not be installed and automatic source restoration failed.",
            correctiveAction: restored
              ? `The revision remains saved at ${receipt.destinationArchivePath}; reopen it explicitly when ready.`
              : `Restart QuickPLS and reopen either the unchanged source or ${receipt.destinationArchivePath}.`,
          }),
          revisionForkStatusMessage: restored
            ? `Revision persisted at ${receipt.destinationArchivePath}; source workspace restored after activation failure.`
            : `Revision persisted at ${receipt.destinationArchivePath}; reopen a validated archive before continuing.`,
        });
        return "blocked";
      }
      useWorkspace.getState().setProjectMeta(
        revisedSnapshot.project.name,
        receipt.destinationArchivePath,
        receipt.projectId,
      );
      set({
        session: {
          kind: "internal_schema6_read_only",
          access: "read_only",
          snapshot: revisedSnapshot,
          originSnapshot: revisedSnapshot,
          project: revisedSnapshot.project,
          standardActivation: {
            modelIds: [receipt.residentModelId],
            sourceArchiveSha256: receipt.destinationArchiveSha256,
          },
          capabilities: INTERNAL_PROJECT_ARCHIVE_V6_SESSION_CAPABILITIES,
        },
        dirty: false,
        persistence: "persisted_validated_archive",
        revisionForkPending: false,
        revisionForkFailure: null,
        revisionForkStatusMessage: `General SEM revision ${receipt.revisionNumber} saved, strictly reopened, and activated at ${receipt.destinationArchivePath}.`,
        standardActivationFailure: null,
        standardActivationStatusMessage: "The revised model and RecipeV4 are the sole active General SEM execution authority.",
        saveCopyFailure: null,
        saveCopyStatusMessage: "General SEM authority revisions use the versioned model-and-Recipe save-as workflow.",
      });
      return "saved";
    },

    forkActiveRecipeBoundRevision: async (options = {}) => {
      const session = get().session;
      const activation = session?.standardActivation;
      if (!session || !activation) {
        set({
          revisionForkFailure: {
            code: "schema6_model_revision.standard_activation_required",
            message: "A strict schema-6 Standard activation is required before creating a revision.",
            correctiveAction: "Open and activate the schema-6 project, then select its RecipeV4-bound model.",
            authoringIssues: [], readinessIssues: [],
          },
          revisionForkStatusMessage: "Model revision blocked because no bound Standard project is active.",
        });
        return "inactive";
      }
      if (supportsGeneralSemV1(session.project)) {
        set({
          revisionForkFailure: {
            code: "schema6_model_revision.general_sem_execution_authority_revision_required",
            message: "General SEM model revisions are disabled because the resident RecipeV4 is the immutable execution authority.",
            correctiveAction: "Keep this archive unchanged until a versioned model-and-recipe execution-authority revision workflow is available.",
            authoringIssues: [], readinessIssues: [],
          },
          revisionForkStatusMessage: "General SEM model revision blocked before any authority mutation.",
        });
        return "blocked";
      }
      if (get().standardActivationPending || get().revisionForkPending || get().saveCopyPending) return "blocked";
      const workspace = useWorkspace.getState();
      const sourceModelId = workspace.activeModelId;
      const captured = workspace.captureStandardSemModelV4SaveAuthorities(activation.modelIds);
      if (!sourceModelId || !captured?.[sourceModelId] || Object.values(captured).some((entry) => entry.dirty)) {
        set({
          revisionForkFailure: {
            code: "schema6_model_revision.clean_source_required",
            message: "Fork-before-edit requires the selected Standard authority set to be clean.",
            correctiveAction: "Restore or reopen the saved RecipeV4-bound authority, then create the revision before editing it.",
            authoringIssues: [], readinessIssues: [],
          },
          revisionForkStatusMessage: "Model revision blocked; no authority was changed.",
        });
        return "blocked";
      }
      const sourceAuthority = captured[sourceModelId].authority;
      const revisionModelId = options.revisionModelId
        ?? `model:revision:${globalThis.crypto.randomUUID()}`;
      const revisionName = options.revisionName
        ?? `${sourceAuthority.model.name} revision`;
      let revision;
      try {
        revision = buildInternalProjectArchiveV6ModelRevisionV1(
          session.project,
          sourceModelId,
          revisionModelId,
          revisionName,
        );
      } catch (error) {
        set({ revisionForkFailure: revisionForkFailureFromUnknown(error), revisionForkStatusMessage: "Model revision blocked; no authority was changed." });
        return "blocked";
      }
      const sourceEpoch = workspace.standardSemModelV4Epochs[sourceModelId];
      const requestEpoch = get().requestEpoch + 1;
      set({
        requestEpoch,
        revisionForkPending: true,
        revisionForkFailure: null,
        revisionForkStatusMessage: "Resolving and appending a new native model authority…",
      });
      try {
        const outcome = await (options.resolver ?? resolveStandardSemModelV4Authority)(revision.revision);
        if (get().requestEpoch !== requestEpoch || get().session !== session) return "stale";
        if (outcome.status === "blocked") {
          set({ revisionForkPending: false, revisionForkFailure: outcome.diagnostic, revisionForkStatusMessage: "Native resolution blocked the revision; both authorities are unchanged." });
          return "blocked";
        }
        const appendedProject = await (options.appender ?? appendResolvedInternalProjectArchiveV6ModelRevision)(
          session.project,
          outcome.value,
        );
        if (get().requestEpoch !== requestEpoch || get().session !== session) return "stale";
        const currentWorkspace = useWorkspace.getState();
        const installed = currentWorkspace.appendStandardSemModelV4Revision({
          sourceModelId,
          expectedSourceModelDocumentSha256: sourceAuthority.model_document_sha256,
          expectedSourceEpoch: sourceEpoch,
        }, {
          authority: parseStandardSemModelV4AuthorityRecordV1({
            schema_version: 1,
            model_document_sha256: outcome.value.modelDocumentSha256,
            model: outcome.value.canonicalModel,
          }),
          layout: { ...captured[sourceModelId].layout, model_id: revisionModelId },
          readiness: outcome.value.readiness,
          scientificSha256: outcome.value.scientificSha256,
        });
        if (!installed) {
          set({ revisionForkPending: false, revisionForkStatusMessage: "Model revision became stale; both persisted authorities remain unchanged." });
          return "stale";
        }
        set({
          session: {
            ...session,
            project: appendedProject,
            standardActivation: {
              ...activation,
              modelIds: [...activation.modelIds, revisionModelId],
            },
          },
          dirty: true,
          persistence: "not_persisted",
          revisionForkPending: false,
          revisionForkFailure: null,
          revisionForkStatusMessage: `Created and activated ${revisionName} (${revisionModelId}). The previous RecipeV4 binding is unchanged.`,
          saveCopyStatusMessage: "The new model revision is not persisted; save a validated new copy.",
        });
        return "activated";
      } catch (error) {
        if (get().requestEpoch !== requestEpoch) return "stale";
        set({
          revisionForkPending: false,
          revisionForkFailure: revisionForkFailureFromUnknown(error),
          revisionForkStatusMessage: "Model revision failed; both authorities and the source archive are unchanged.",
        });
        return "blocked";
      }
    },

    saveCopy: async (executor = saveInternalProjectArchiveV6Copy) => {
      const session = get().session;
      if (!session) {
        set({
          saveCopyFailure: {
            code: "schema6_save_copy.session_required",
            message: "No isolated schema-6 session is active.",
            correctiveAction: "Open a strict schema-6 archive in Experimental Labs first.",
          },
          saveCopyStatusMessage: "Save copy was blocked because no schema-6 session is active.",
        });
        return "inactive";
      }
      if (supportsGeneralSemV1(session.project)) {
        set({
          saveCopyFailure: {
            code: "schema6_save_copy.general_sem_execution_authority_revision_required",
            message: "General SEM Save copy is disabled because it cannot yet revise the resident model and RecipeV4 execution authority together.",
            correctiveAction: "Keep this validated archive unchanged until the versioned General SEM execution-authority revision workflow is available.",
          },
          saveCopyStatusMessage: "General SEM Save copy blocked before candidate derivation or native file selection.",
        });
        return "blocked";
      }
      if (!session.standardActivation) {
        set({
          saveCopyFailure: {
            code: "schema6_save_copy.standard_activation_required",
            message: "The schema-6 models have not been activated as Standard authorities.",
            correctiveAction: "Activate Standard authority, make edits there, then save a validated new copy.",
          },
          saveCopyStatusMessage: "Save copy was blocked until Standard authority is activated.",
        });
        return "blocked";
      }
      if (get().standardActivationPending || get().revisionForkPending || get().saveCopyPending) {
        set({
          saveCopyFailure: {
            code: "schema6_save_copy.operation_pending",
            message: "Another schema-6 session operation is still running.",
            correctiveAction: "Wait for the current operation to finish, then choose Save copy again.",
          },
          saveCopyStatusMessage: "Save copy was not started while another session operation was pending.",
        });
        return "blocked";
      }

      const captured = useWorkspace.getState().captureStandardSemModelV4SaveAuthorities(
        session.standardActivation.modelIds,
      );
      if (!captured) {
        set({
          saveCopyFailure: {
            code: "schema6_save_copy.standard_authority_stale",
            message: "The activated Standard authority set no longer matches this schema-6 session.",
            correctiveAction: "Reopen and reactivate the source archive before saving a copy.",
          },
          saveCopyStatusMessage: "Save copy was blocked because its Standard authority binding is stale.",
        });
        return "stale";
      }
      let candidate: InternalProjectArchiveV6Wire;
      try {
        candidate = deriveInternalProjectArchiveV6StandardSaveCandidateV1(session.project, captured);
      } catch (error) {
        set({
          saveCopyFailure: saveCopyFailureFromUnknown(error),
          saveCopyStatusMessage: "Save copy was blocked while deriving the strict Standard authority candidate.",
        });
        return "blocked";
      }

      const requestEpoch = get().requestEpoch + 1;
      set({
        requestEpoch,
        saveCopyPending: true,
        saveCopyFailure: null,
        saveCopyStatusMessage: "Choose a new destination for the validated schema-6 copy…",
      });

      try {
        const outcome = await executor(session.snapshot, candidate);
        if (get().requestEpoch !== requestEpoch) return "stale";
        if (outcome === null) {
          set({
            saveCopyPending: false,
            saveCopyFailure: null,
            saveCopyStatusMessage: "Save copy cancelled before a native write started; the session is unchanged.",
          });
          return "cancelled";
        }
        if (outcome.status === "blocked") {
          set({
            saveCopyPending: false,
            saveCopyFailure: outcome.diagnostic,
            saveCopyStatusMessage: "Save copy was blocked; the current session and source archive are unchanged.",
          });
          return "blocked";
        }

        const current = get().session;
        if (!current || current !== session) return "stale";
        if (!useWorkspace.getState().reanchorStandardSemModelV4Authorities(captured)) {
          set({
            saveCopyPending: false,
            saveCopyFailure: null,
            saveCopyStatusMessage: "A validated copy was written, but newer Standard changes made this response stale; current state was not reanchored.",
          });
          return "stale";
        }
        set({
          session: {
            ...current,
            snapshot: outcome.value.snapshot,
            project: outcome.value.snapshot.project,
            standardActivation: current.standardActivation
              ? {
                  ...current.standardActivation,
                  sourceArchiveSha256: outcome.value.snapshot.archiveSha256,
                }
              : null,
          },
          dirty: false,
          persistence: outcome.value.persistence,
          saveCopyPending: false,
          saveCopyFailure: null,
          saveCopyStatusMessage: `Validated new copy saved at ${outcome.value.receipt.destinationArchivePath}.`,
          modelMutationFailure: null,
          modelMutationStatusMessage: "Standard authority is now based on the validated saved copy.",
        });
        return "saved";
      } catch (error) {
        if (get().requestEpoch !== requestEpoch) return "stale";
        set({
          saveCopyPending: false,
          saveCopyFailure: saveCopyFailureFromUnknown(error),
          saveCopyStatusMessage: "Save copy failed; the current session and source archive are unchanged.",
        });
        return "blocked";
      }
    },

    reanchorGeneralSemSnapshot: (snapshot) => {
      const state = get();
      const session = state.session;
      const activation = session?.standardActivation;
      if (!session || !activation) return "inactive";
      if (state.standardActivationPending || state.revisionForkPending || state.saveCopyPending || state.dirty) {
        set({
          standardActivationFailure: {
            code: "schema6_general_sem_reanchor.operation_pending_or_dirty",
            message: "The General SEM project cannot be reanchored while another authority operation or unsaved model change exists.",
            correctiveAction: "Wait for the current operation or restore the exact clean project authority, then verify the archive again.",
            authoringIssues: [],
            readinessIssues: [],
          },
          standardActivationStatusMessage: "General SEM archive reanchor was blocked; the current source binding is unchanged.",
        });
        return "blocked";
      }
      const previous = session.snapshot.generalSemExecutionAuthority;
      const next = snapshot.generalSemExecutionAuthority;
      const sameModelIds = [...activation.modelIds].sort().join("\0")
        === snapshot.project.models.map((record) => record.model_id).sort().join("\0");
      if (!supportsGeneralSemV1(session.project)
        || !supportsGeneralSemV1(snapshot.project)
        || session.snapshot.archivePath !== snapshot.archivePath
        || session.project.project_id !== snapshot.project.project_id
        || !previous
        || !next
        || previous.projectId !== next.projectId
        || previous.datasetId !== next.datasetId
        || previous.datasetFingerprint !== next.datasetFingerprint
        || previous.modelId !== next.modelId
        || previous.modelScientificSha256 !== next.modelScientificSha256
        || previous.recipeId !== next.recipeId
        || previous.recipeDocumentSha256 !== next.recipeDocumentSha256
        || !sameModelIds) {
        set({
          standardActivationFailure: {
            code: "schema6_general_sem_reanchor.authority_mismatch",
            message: "The strictly reopened archive differs from the active General SEM project authority.",
            correctiveAction: "Preserve both files unchanged and do not display, export, or append results from this archive.",
            authoringIssues: [],
            readinessIssues: [],
          },
          standardActivationStatusMessage: "General SEM archive reanchor failed closed; the previous source binding remains active.",
        });
        return "blocked";
      }
      const captured = useWorkspace.getState().captureStandardSemModelV4SaveAuthorities(activation.modelIds);
      if (!captured
        || Object.values(captured).some((item) => item.dirty)
        || captured[next.modelId]?.scientificSha256 !== next.modelScientificSha256) {
        set({
          standardActivationFailure: {
            code: "schema6_general_sem_reanchor.workspace_authority_stale",
            message: "The active canvas authority no longer matches the strictly reopened General SEM archive.",
            correctiveAction: "Keep the current project open and resolve the stale model authority before continuing.",
            authoringIssues: [],
            readinessIssues: [],
          },
          standardActivationStatusMessage: "General SEM archive reanchor failed closed; the canvas binding is unchanged.",
        });
        return "blocked";
      }
      set({
        session: {
          ...session,
          snapshot,
          project: snapshot.project,
          standardActivation: {
            ...activation,
            sourceArchiveSha256: snapshot.archiveSha256,
          },
        },
        dirty: false,
        persistence: "persisted_validated_archive",
        standardActivationFailure: null,
        standardActivationStatusMessage: "The active General SEM project was reanchored to the strictly validated current archive.",
      });
      return "reanchored";
    },

    closeStandardProject: () => {
      const state = get();
      const activation = state.session?.standardActivation;
      if (!state.session || !activation) return "inactive";
      if (
        state.standardActivationPending
        || state.revisionForkPending
        || state.saveCopyPending
        || state.dirty
        || (state.persistence !== "persisted_new_copy" && state.persistence !== "persisted_validated_archive")
      ) {
        set({
          standardActivationFailure: {
            code: "schema6_standard_activation.close_requires_clean_saved_copy",
            message: "The bound Standard project cannot be closed yet.",
            correctiveAction: state.standardActivationPending || state.saveCopyPending
              ? "Wait for the current schema-6 operation to finish, then try again."
              : "Save a validated new copy with no newer Standard changes, then choose Close Standard project.",
            authoringIssues: [],
            readinessIssues: [],
          },
          standardActivationStatusMessage: "Close blocked; the Standard workspace and schema-6 source binding are unchanged.",
        });
        return "blocked";
      }
      if (!useWorkspace.getState().clearStandardSemModelV4Workspace(activation.modelIds)) {
        set({
          standardActivationFailure: {
            code: "schema6_standard_activation.close_binding_stale",
            message: "The Standard authority set no longer matches this schema-6 source binding.",
            correctiveAction: "Keep the session open and resolve the stale authority state before closing.",
            authoringIssues: [],
            readinessIssues: [],
          },
          standardActivationStatusMessage: "Close failed closed; the schema-6 source session remains bound.",
        });
        return "blocked";
      }
      set({
        phase: "inactive",
        session: null,
        failure: null,
        statusMessage: "Standard project closed and its schema-6 source binding was released.",
        requestEpoch: state.requestEpoch + 1,
        dirty: false,
        persistence: null,
        modelMutationPending: false,
        modelMutationFailure: null,
        modelMutationStatusMessage: "Detached schema-6 model mutation remains disabled.",
        standardActivationPending: false,
        standardActivationFailure: null,
        standardActivationStatusMessage: "No schema-6 source session is associated with the Standard workspace.",
        revisionForkPending: false,
        revisionForkFailure: null,
        revisionForkStatusMessage: "No RecipeV4-bound model revision is active.",
        saveCopyPending: false,
        saveCopyFailure: null,
        saveCopyStatusMessage: "No schema-6 copy is active after closing the Standard project.",
      });
      return "closed";
    },

    deactivate: () => set((state) => state.session?.standardActivation
      ? {
          standardActivationFailure: {
            code: "schema6_standard_activation.source_session_required",
            message: "The active Standard authority is still bound to this schema-6 source session.",
            correctiveAction: state.dirty
              ? "Save a validated new copy before attempting to close this Labs source session."
              : "Keep this source session open while its Standard authority is active.",
            authoringIssues: [],
            readinessIssues: [],
          },
          standardActivationStatusMessage: "Close blocked so the Standard authority cannot be detached from its schema-6 source.",
          statusMessage: "Schema-6 session remains active because Standard authority is bound to it.",
        }
      : {
          phase: "inactive",
          session: null,
          failure: null,
          statusMessage: "Schema-6 read-only session closed. Its in-memory snapshot was released.",
          requestEpoch: state.requestEpoch + 1,
          dirty: false,
          persistence: null,
          modelMutationPending: false,
          modelMutationFailure: null,
          modelMutationStatusMessage: "Detached schema-6 model mutation remains disabled.",
          standardActivationPending: false,
          standardActivationFailure: null,
          standardActivationStatusMessage: "No schema-6 source session is associated with the current Standard project.",
          revisionForkPending: false,
          revisionForkFailure: null,
          revisionForkStatusMessage: "No RecipeV4-bound model revision is active.",
          saveCopyPending: false,
          saveCopyFailure: null,
          saveCopyStatusMessage: "No schema-6 copy is active after closing the session.",
        }),
  }));

useWorkspace.subscribe(() => {
  const sessionState = useInternalProjectArchiveV6Session.getState();
  const activation = sessionState.session?.standardActivation;
  if (!activation || sessionState.saveCopyPending) return;
  const captured = useWorkspace.getState().captureStandardSemModelV4SaveAuthorities(activation.modelIds);
  if (!captured) return;
  const structuralDirty = sessionState.session?.project !== sessionState.session?.snapshot.project;
  const dirty = structuralDirty || Object.values(captured).some((authority) => authority.dirty);
  if (dirty !== sessionState.dirty) {
    useInternalProjectArchiveV6Session.setState({
      dirty,
      persistence: dirty ? "not_persisted" : sessionState.persistence,
    });
  }
});
