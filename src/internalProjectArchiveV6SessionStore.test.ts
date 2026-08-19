import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { InternalProjectArchiveV6ModelMutationOutcomeV1 } from "./domain/internalProjectArchiveV6ModelMutation";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "./domain/internalProjectArchiveV6Read";
import { parseInternalProjectArchiveV6Wire, type InternalProjectArchiveV6Wire } from "./domain/internalProjectArchiveV6Wire";
import { convertLegacyBasicModelV4, type LegacyBasicModelV4Input } from "./domain/semModelV4";
import type { StandardSemModelV4AuthorityResolveOutcomeV1 } from "./domain/standardSemModelV4AuthorityCas";
import { useWorkspace } from "./store";
import {
  INTERNAL_PROJECT_ARCHIVE_V6_SESSION_CAPABILITIES,
  useInternalProjectArchiveV6Session,
} from "./internalProjectArchiveV6SessionStore";

const MODEL_DOCUMENT_SHA256 = "c".repeat(64);
const DATASET_ID = "00000000-0000-0000-0000-000000000602";

const semInput: LegacyBasicModelV4Input = {
  id: "model:draft:1",
  name: "Draft SEM model",
  constructs: [
    { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
    { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
  ],
  paths: [{ source: "x", target: "y" }],
  controls: [],
  higher_order_constructs: [],
  interactions: [],
};

const convertedSemModel = convertLegacyBasicModelV4(semInput, "cbsem_common_factor");
const semModel = {
  ...convertedSemModel,
  data_binding: { ...convertedSemModel.data_binding, dataset_id: DATASET_ID },
};
const project = parseInternalProjectArchiveV6Wire({
  schema_version: 6,
  project_id: "00000000-0000-0000-0000-000000000601",
  name: "Schema-6 study",
  created_at: "2026-08-15T10:00:00Z",
  modified_at: "2026-08-15T10:01:00Z",
  origin: { kind: "new_project" },
  datasets: [{
    id: DATASET_ID,
    name: "Descriptor-only data",
    schema: {
      version: 1,
      kind: "raw",
      columns: ["x1", "x2", "y1", "y2"].map((name) => ({
        name,
        label: null,
        column_type: "numeric",
        scale_type: "continuous",
        missing_markers: ["NA"],
        theoretical_min: null,
        theoretical_max: null,
        value_labels: {},
      })),
      case_count: 123,
      sample_size: null,
    },
    fingerprint: "schema6-dataset-fingerprint",
  }],
  models: [{
    model_id: semModel.id,
    payload: {
      kind: "sem_model_v4_draft",
      model: semModel,
      model_document_sha256: MODEL_DOCUMENT_SHA256,
    },
  }],
});

const snapshot = {
  schemaVersion: 1,
  access: "read_only",
  loader: "strict_schema6_zip",
  archivePath: "D:\\projects\\study-v6.qpls",
  archiveSha256: "a".repeat(64),
  archiveBytes: 12_345,
  manifest: {
    schema_version: 6,
    project_id: "00000000-0000-0000-0000-000000000601",
    name: "Schema-6 study",
    created_at: "2026-08-15T10:00:00Z",
    modified_at: "2026-08-15T10:01:00Z",
    engine_version: "quickpls-test",
    checksum_algorithm: "sha256",
    checksums: { "project.json": "b".repeat(64) },
  },
  project,
  residentDatasets: [],
  counts: {
    datasets: 1,
    models: 1,
    recipes: 2,
    historicalRecipes: 3,
    historicalResults: 4,
    canonicalResultDocuments: 5,
  },
  sourceRecheckedUnchanged: true,
} as unknown as InternalProjectArchiveV6ReadSnapshotV1;

const resolvedAuthority = (model = semModel): StandardSemModelV4AuthorityResolveOutcomeV1 => ({
  status: "ok",
  value: {
    schemaVersion: 1,
    canonicalModel: model,
    modelDocumentSha256: MODEL_DOCUMENT_SHA256,
    scientificSha256: null,
    readiness: "authoring_only",
    authoringIssues: [],
    readinessIssues: [],
  },
});

async function openAndActivate() {
  await useInternalProjectArchiveV6Session.getState().open(async () => ({ status: "ok", value: snapshot }));
  await expect(useInternalProjectArchiveV6Session.getState().activateStandardAuthorities(async () => resolvedAuthority()))
    .resolves.toBe("activated");
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => { resolve = next; });
  return { promise, resolve };
}

describe("Internal/Labs schema-6 in-memory session store", () => {
  beforeEach(() => {
    useInternalProjectArchiveV6Session.setState((state) => ({
      phase: "inactive",
      session: null,
      requestEpoch: state.requestEpoch + 1,
      dirty: false,
      persistence: null,
      standardActivationPending: false,
      standardActivationFailure: null,
      revisionForkPending: false,
      revisionForkFailure: null,
      saveCopyPending: false,
    }));
    useWorkspace.getState().resetProject();
  });

  afterEach(() => {
    useWorkspace.getState().resetProject();
  });

  it("activates the full strict-reader snapshot without changing the Standard project", async () => {
    useWorkspace.getState().setProjectMeta("Standard project", "D:\\projects\\standard.qpls");
    const standardBefore = {
      projectName: useWorkspace.getState().projectName,
      projectPath: useWorkspace.getState().projectPath,
      projectWritable: useWorkspace.getState().projectWritable,
    };

    await expect(useInternalProjectArchiveV6Session.getState().open(async () => ({
      status: "ok",
      value: snapshot,
    }))).resolves.toBe("activated");
    const state = useInternalProjectArchiveV6Session.getState();
    expect(state.phase).toBe("active");
    expect(state.session?.snapshot).toBe(snapshot);
    expect(state.session?.project).toBe(snapshot.project);
    expect(state.session?.access).toBe("read_only");
    expect(state.session?.capabilities).toEqual({
      edit: false,
      ephemeralModelAuthorityMutation: false,
      compile: false,
      run: false,
      save: false,
      saveAs: false,
      saveCopy: "new_destination_only",
      autosave: false,
      recovery: false,
    });
    expect(Object.isFrozen(INTERNAL_PROJECT_ARCHIVE_V6_SESSION_CAPABILITIES)).toBe(true);
    expect({
      projectName: useWorkspace.getState().projectName,
      projectPath: useWorkspace.getState().projectPath,
      projectWritable: useWorkspace.getState().projectWritable,
    }).toEqual(standardBefore);
  });

  it("reanchors an exact marked archive snapshot and then closes the clean project", async () => {
    await openAndActivate();
    const digest = "d".repeat(64);
    useWorkspace.setState((state) => ({
      standardSemModelV4Persistence: {
        ...state.standardSemModelV4Persistence,
        [semModel.id]: {
          ...state.standardSemModelV4Persistence[semModel.id],
          scientificSha256: digest,
        },
      },
    }));
    const current = useInternalProjectArchiveV6Session.getState().session;
    if (!current?.standardActivation) throw new Error("Expected an activated schema-6 session.");
    const recipeId = "00000000-0000-4000-8000-000000000699";
    const executionAuthority = {
      schemaVersion: 1 as const,
      projectId: project.project_id,
      datasetId: DATASET_ID,
      datasetFingerprint: project.datasets[0].fingerprint,
      modelId: semModel.id,
      modelScientificSha256: digest,
      recipeId,
      recipeDocumentSha256: "e".repeat(64),
      recipe: {} as never,
    };
    const markedProject = {
      ...project,
      sem_generation: "general_sem_v1" as const,
    };
    const markedSnapshot = {
      ...snapshot,
      project: markedProject,
      generalSemExecutionAuthority: executionAuthority,
    };
    useInternalProjectArchiveV6Session.setState({
      session: {
        ...current,
        snapshot: markedSnapshot,
        project: markedProject,
      },
      dirty: false,
      persistence: "persisted_validated_archive",
    });
    const next = {
      ...markedSnapshot,
      archiveSha256: "f".repeat(64),
    };

    expect(useInternalProjectArchiveV6Session.getState().reanchorGeneralSemSnapshot(next))
      .toBe("reanchored");
    expect(useInternalProjectArchiveV6Session.getState().session?.snapshot.archiveSha256)
      .toBe("f".repeat(64));
    expect(useInternalProjectArchiveV6Session.getState().closeStandardProject()).toBe("closed");
    expect(useInternalProjectArchiveV6Session.getState().session).toBeNull();
    expect(useWorkspace.getState().activeModelId).toBeNull();
  });

  it("revokes backend draft authority before resolving any marked Standard model", async () => {
    const markedSnapshot = {
      ...snapshot,
      project: { ...project, sem_generation: "general_sem_v1" as const },
    };
    await useInternalProjectArchiveV6Session.getState().open(async () => ({
      status: "ok",
      value: markedSnapshot,
    }));
    const order: string[] = [];
    const result = await useInternalProjectArchiveV6Session.getState().activateStandardAuthorities(
      async () => {
        order.push("resolve");
        return resolvedAuthority();
      },
      async () => { order.push("revoke"); },
    );

    expect(result).toBe("activated");
    expect(order).toEqual(["revoke", "resolve"]);
    expect(useInternalProjectArchiveV6Session.getState().persistence).toBe("persisted_validated_archive");
    expect(useWorkspace.getState().projectWritable).toBe(false);
  });

  it("blocks detached mutation without invoking its executor or changing either authority", async () => {
    await useInternalProjectArchiveV6Session.getState().open(async () => ({
      status: "ok",
      value: snapshot,
    }));
    const sourceProject = useInternalProjectArchiveV6Session.getState().session!.project;
    const executor = vi.fn(async (): Promise<InternalProjectArchiveV6ModelMutationOutcomeV1> => ({
      status: "ok",
      value: {
        schemaVersion: 1,
        persistence: "not_persisted",
        project: sourceProject,
      },
    }));
    const mutation = {
      kind: "promote_draft" as const,
      modelId: "model:draft:1",
      expectedModelDocumentSha256: "c".repeat(64),
    };

    await expect(useInternalProjectArchiveV6Session.getState().mutateModel(mutation, executor))
      .resolves.toBe("blocked");

    const state = useInternalProjectArchiveV6Session.getState();
    expect(executor).not.toHaveBeenCalled();
    expect(state.session?.snapshot).toBe(snapshot);
    expect(state.session?.project).toBe(sourceProject);
    expect(state).toMatchObject({
      dirty: false,
      persistence: null,
      modelMutationPending: false,
      modelMutationFailure: {
        code: "schema6_model_mutation.standard_authority_required",
      },
    });

    useInternalProjectArchiveV6Session.getState().deactivate();
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      session: null,
      dirty: false,
      persistence: null,
    });
  });

  it("forks a clean RecipeV4-bound model under a new authority and leaves the recipe unchanged", async () => {
    const sourceScientific = "d".repeat(64);
    const revisionDocument = "e".repeat(64);
    const revisionScientific = "f".repeat(64);
    const readyProject = {
      ...project,
      models: [{
        model_id: semModel.id,
        payload: { kind: "sem_model_v4" as const, model: semModel, scientific_sha256: sourceScientific },
      }],
      recipes: [{
        schema_version: 4,
        id: "00000000-0000-0000-0000-000000000699",
        model_binding: {
          kind: "project_sem_model_v4_reference" as const,
          model_id: semModel.id,
          scientific_sha256: sourceScientific,
        },
      }] as InternalProjectArchiveV6Wire["recipes"],
    } as InternalProjectArchiveV6Wire;
    const readySnapshot = { ...snapshot, project: readyProject };
    await useInternalProjectArchiveV6Session.getState().open(async () => ({ status: "ok", value: readySnapshot }));
    await expect(useInternalProjectArchiveV6Session.getState().activateStandardAuthorities(async (model) => ({
      status: "ok",
      value: {
        schemaVersion: 1,
        canonicalModel: model,
        modelDocumentSha256: MODEL_DOCUMENT_SHA256,
        scientificSha256: sourceScientific,
        readiness: "ready",
        authoringIssues: [],
        readinessIssues: [],
      },
    }))).resolves.toBe("activated");
    expect(useWorkspace.getState().standardSemModelV4ScientificEditLocks).toEqual({ [semModel.id]: true });
    await expect(useWorkspace.getState().commitStandardSemModelV4Intent({
      kind: "set_model_name",
      name: "Forbidden source edit",
    })).resolves.toMatchObject({
      status: "blocked",
      diagnostic: { code: "schema6_standard_authority.scientific_revision_fork_required" },
    });

    const originalRecipe = readyProject.recipes[0];
    const revisionId = "model:revision:2";
    const appender = vi.fn(async (_source: InternalProjectArchiveV6Wire, resolved: { canonicalModel: typeof semModel }) => ({
      ...readyProject,
      models: [...readyProject.models, {
        model_id: revisionId,
        payload: { kind: "sem_model_v4" as const, model: resolved.canonicalModel, scientific_sha256: revisionScientific },
      }],
    }));
    await expect(useInternalProjectArchiveV6Session.getState().forkActiveRecipeBoundRevision({
      revisionModelId: revisionId,
      revisionName: "Draft SEM model revision 2",
      resolver: async (model) => ({
        status: "ok",
        value: {
          schemaVersion: 1,
          canonicalModel: model,
          modelDocumentSha256: revisionDocument,
          scientificSha256: revisionScientific,
          readiness: "ready",
          authoringIssues: [],
          readinessIssues: [],
        },
      }),
      appender: appender as never,
    })).resolves.toBe("activated");

    const state = useInternalProjectArchiveV6Session.getState();
    expect(state.session?.project.recipes[0]).toBe(originalRecipe);
    expect(state.session?.standardActivation?.modelIds).toEqual([semModel.id, revisionId]);
    expect(state.dirty).toBe(true);
    expect(state.persistence).toBe("not_persisted");
    expect(useWorkspace.getState().activeModelId).toBe(revisionId);
    expect(useWorkspace.getState().explorerSelection).toEqual({ kind: "model", modelId: revisionId });
    expect(useWorkspace.getState().standardSemModelV4ScientificEditLocks).toEqual({ [semModel.id]: true });
    expect(useWorkspace.getState().standardSemModelV4Authorities[semModel.id]).toMatchObject({
      model_document_sha256: MODEL_DOCUMENT_SHA256,
      model: { id: semModel.id, name: semModel.name },
    });
    expect(appender).toHaveBeenCalledTimes(1);
  });

  it("does not let an injected mutation executor replace the fail-closed authority diagnostic", async () => {
    await useInternalProjectArchiveV6Session.getState().open(async () => ({
      status: "ok",
      value: snapshot,
    }));
    const sourceProject = useInternalProjectArchiveV6Session.getState().session!.project;
    const diagnostic = {
      code: "schema6_model_mutation.stale_model_digest",
      message: "The draft digest is stale.",
      correctiveAction: "Refresh and retry.",
    };

    const executor = vi.fn(async () => ({ status: "blocked" as const, diagnostic }));
    await expect(useInternalProjectArchiveV6Session.getState().mutateModel({
      kind: "promote_draft",
      modelId: "model:draft:1",
      expectedModelDocumentSha256: "c".repeat(64),
    }, executor)).resolves.toBe("blocked");

    expect(executor).not.toHaveBeenCalled();
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      dirty: false,
      persistence: null,
      modelMutationPending: false,
      modelMutationFailure: {
        code: "schema6_model_mutation.standard_authority_required",
      },
    });
    expect(useInternalProjectArchiveV6Session.getState().session?.project).toBe(sourceProject);
  });

  it("reanchors the base and current snapshot only after a validated new-copy receipt", async () => {
    await useInternalProjectArchiveV6Session.getState().open(async () => ({
      status: "ok",
      value: snapshot,
    }));
    const preActivationExecutor = vi.fn();
    await expect(useInternalProjectArchiveV6Session.getState().saveCopy(preActivationExecutor))
      .resolves.toBe("blocked");
    expect(preActivationExecutor).not.toHaveBeenCalled();
    expect(useInternalProjectArchiveV6Session.getState().saveCopyFailure?.code)
      .toBe("schema6_save_copy.standard_activation_required");

    await expect(useInternalProjectArchiveV6Session.getState().activateStandardAuthorities(async () => resolvedAuthority()))
      .resolves.toBe("activated");
    let destinationSnapshot: InternalProjectArchiveV6ReadSnapshotV1 | null = null;
    let savedCandidate: InternalProjectArchiveV6Wire | null = null;
    const executor = vi.fn(async (
      sourceSnapshot: InternalProjectArchiveV6ReadSnapshotV1,
      candidate: InternalProjectArchiveV6Wire,
    ) => {
      savedCandidate = candidate;
      destinationSnapshot = {
        ...sourceSnapshot,
        archivePath: "D:\\projects\\study-v6-model-copy.qpls",
        archiveSha256: "d".repeat(64),
        archiveBytes: 23_456,
        project: candidate,
        counts: { ...sourceSnapshot.counts, models: candidate.models.length },
      };
      return {
        status: "ok" as const,
        value: {
          schemaVersion: 1 as const,
          persistence: "persisted_new_copy" as const,
          receipt: {
            schemaVersion: 1 as const,
            sourceArchivePath: sourceSnapshot.archivePath,
            sourceArchiveSha256: sourceSnapshot.archiveSha256,
            sourceVerifiedUnchanged: true as const,
            destinationArchivePath: destinationSnapshot.archivePath,
            destinationArchiveSha256: destinationSnapshot.archiveSha256,
            destinationArchiveBytes: destinationSnapshot.archiveBytes,
            strictReopenValidated: true as const,
            modelCount: candidate.models.length,
          },
          snapshot: destinationSnapshot,
        },
      };
    });

    await expect(useInternalProjectArchiveV6Session.getState().saveCopy(executor))
      .resolves.toBe("saved");

    const state = useInternalProjectArchiveV6Session.getState();
    expect(executor).toHaveBeenCalledWith(snapshot, savedCandidate);
    expect(state.session?.snapshot).toBe(destinationSnapshot);
    expect(state.session?.originSnapshot).toBe(snapshot);
    expect(state.session?.project).toBe(savedCandidate);
    expect(state.session?.standardActivation?.sourceArchiveSha256).toBe("d".repeat(64));
    expect(state).toMatchObject({
      dirty: false,
      persistence: "persisted_new_copy",
      saveCopyPending: false,
      saveCopyFailure: null,
    });
  });

  it("keeps the activated source and Standard binding unchanged on chooser cancel or collision", async () => {
    await openAndActivate();
    const before = useInternalProjectArchiveV6Session.getState().session;

    await expect(useInternalProjectArchiveV6Session.getState().saveCopy(async () => null))
      .resolves.toBe("cancelled");
    expect(useInternalProjectArchiveV6Session.getState().session).toBe(before);
    expect(useInternalProjectArchiveV6Session.getState().dirty).toBe(false);

    const diagnostic = {
      code: "schema6_save_copy.destination_exists",
      message: "Destination exists.",
      correctiveAction: "Choose a new filename.",
    };
    await expect(useInternalProjectArchiveV6Session.getState().saveCopy(async () => ({
      status: "blocked",
      diagnostic,
    }))).resolves.toBe("blocked");
    expect(useInternalProjectArchiveV6Session.getState().session).toBe(before);
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      dirty: false,
      persistence: null,
      saveCopyPending: false,
      saveCopyFailure: diagnostic,
    });
  });

  it("projects only the bound schema-6 dataset descriptor and blocks Close while Standard remains activated", async () => {
    await openAndActivate();
    const workspace = useWorkspace.getState();
    expect(workspace.datasetDescriptorOnly).toBe(true);
    expect(workspace.dataset).toMatchObject({
      id: DATASET_ID,
      name: "Descriptor-only data",
      columns: ["x1", "x2", "y1", "y2"],
      rows: [],
      rowCount: 123,
      fingerprint: "schema6-dataset-fingerprint",
      kind: "raw",
      sampleSize: null,
    });
    expect(Number.isNaN(workspace.dataset.missing)).toBe(true);

    const node = workspace.nodes[0];
    workspace.onNodesChange([{
      id: node.id,
      type: "position",
      position: { x: node.position.x + 10, y: node.position.y + 10 },
    }]);
    expect(useInternalProjectArchiveV6Session.getState().dirty).toBe(true);
    const bound = useInternalProjectArchiveV6Session.getState().session;
    useInternalProjectArchiveV6Session.getState().deactivate();
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      phase: "active",
      session: bound,
      dirty: true,
      standardActivationFailure: { code: "schema6_standard_activation.source_session_required" },
    });
  });

  it("blocks marked General SEM model fork and Save copy before any authority or native mutation", async () => {
    await openAndActivate();
    const current = useInternalProjectArchiveV6Session.getState().session;
    if (!current?.standardActivation) throw new Error("Expected an activated schema-6 session.");
    const markedProject = { ...current.project, sem_generation: "general_sem_v1" as const };
    const markedSession = { ...current, project: markedProject };
    useInternalProjectArchiveV6Session.setState({
      session: markedSession,
      dirty: false,
      persistence: "persisted_validated_archive",
    });
    const workspaceBefore = useWorkspace.getState();
    const projectBefore = structuredClone(markedProject);
    const resolver = vi.fn(async () => resolvedAuthority());
    const appender = vi.fn(async () => markedProject);
    const saveExecutor = vi.fn();

    await expect(useInternalProjectArchiveV6Session.getState().forkActiveRecipeBoundRevision({
      revisionModelId: "model:must-not-exist",
      resolver,
      appender,
    })).resolves.toBe("blocked");
    await expect(useInternalProjectArchiveV6Session.getState().saveCopy(saveExecutor))
      .resolves.toBe("blocked");

    expect(resolver).not.toHaveBeenCalled();
    expect(appender).not.toHaveBeenCalled();
    expect(saveExecutor).not.toHaveBeenCalled();
    expect(useWorkspace.getState()).toBe(workspaceBefore);
    expect(useInternalProjectArchiveV6Session.getState().session?.project).toEqual(projectBefore);
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      dirty: false,
      persistence: "persisted_validated_archive",
      revisionForkPending: false,
      revisionForkFailure: { code: "schema6_model_revision.general_sem_execution_authority_revision_required" },
      saveCopyPending: false,
      saveCopyFailure: { code: "schema6_save_copy.general_sem_execution_authority_revision_required" },
    });
  });

  it("closes the clean saved Standard workspace and source binding together", async () => {
    await openAndActivate();
    const authorityIds = Object.keys(useWorkspace.getState().standardSemModelV4Authorities);

    expect(useInternalProjectArchiveV6Session.getState().closeStandardProject()).toBe("blocked");
    expect(useWorkspace.getState().standardSemModelV4Authorities).not.toEqual({});

    useInternalProjectArchiveV6Session.setState({ saveCopyPending: true });
    expect(useInternalProjectArchiveV6Session.getState().closeStandardProject()).toBe("blocked");
    useInternalProjectArchiveV6Session.setState({ saveCopyPending: false });

    const node = useWorkspace.getState().nodes[0];
    useWorkspace.getState().onNodesChange([{
      id: node.id,
      type: "position",
      position: { x: node.position.x + 10, y: node.position.y + 10 },
    }]);
    expect(useInternalProjectArchiveV6Session.getState().dirty).toBe(true);
    expect(useInternalProjectArchiveV6Session.getState().closeStandardProject()).toBe("blocked");

    await expect(useInternalProjectArchiveV6Session.getState().saveCopy(async (sourceSnapshot, candidate) => {
      const destination = {
        ...sourceSnapshot,
        archivePath: "D:\\projects\\study-v6-clean-copy.qpls",
        archiveSha256: "e".repeat(64),
        project: candidate,
      };
      return {
        status: "ok" as const,
        value: {
          schemaVersion: 1 as const,
          persistence: "persisted_new_copy" as const,
          receipt: {
            schemaVersion: 1 as const,
            sourceArchivePath: sourceSnapshot.archivePath,
            sourceArchiveSha256: sourceSnapshot.archiveSha256,
            sourceVerifiedUnchanged: true as const,
            destinationArchivePath: destination.archivePath,
            destinationArchiveSha256: destination.archiveSha256,
            destinationArchiveBytes: destination.archiveBytes,
            strictReopenValidated: true as const,
            modelCount: candidate.models.length,
          },
          snapshot: destination,
        },
      };
    })).resolves.toBe("saved");

    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      dirty: false,
      persistence: "persisted_new_copy",
    });
    expect(useInternalProjectArchiveV6Session.getState().closeStandardProject()).toBe("closed");
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      phase: "inactive",
      session: null,
      dirty: false,
      persistence: null,
    });
    expect(useWorkspace.getState()).toMatchObject({
      activeModelId: null,
      standardSemModelV4Authorities: {},
      view: "welcome",
    });
    expect(authorityIds).toEqual([semModel.id]);
  });

  it("ignores a late Standard-activation response after explicit close", async () => {
    await useInternalProjectArchiveV6Session.getState().open(async () => ({
      status: "ok",
      value: snapshot,
    }));
    const pending = deferred<StandardSemModelV4AuthorityResolveOutcomeV1>();
    const activating = useInternalProjectArchiveV6Session.getState().activateStandardAuthorities(() => pending.promise);
    expect(useInternalProjectArchiveV6Session.getState().standardActivationPending).toBe(true);

    useInternalProjectArchiveV6Session.getState().deactivate();
    pending.resolve(resolvedAuthority());

    await expect(activating).resolves.toBe("stale");
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      phase: "inactive",
      session: null,
      dirty: false,
      persistence: null,
      standardActivationPending: false,
    });
    expect(useWorkspace.getState().standardSemModelV4Authorities).not.toHaveProperty(semModel.id);
  });

  it("keeps an existing session active when a replacement is cancelled or blocked", async () => {
    const store = useInternalProjectArchiveV6Session.getState();
    await store.open(async () => ({ status: "ok", value: snapshot }));

    await expect(useInternalProjectArchiveV6Session.getState().open(async () => null))
      .resolves.toBe("cancelled");
    expect(useInternalProjectArchiveV6Session.getState().session?.snapshot).toBe(snapshot);

    const diagnostic = {
      code: "schema6_archive_read.invalid_archive",
      message: "Archive validation failed.",
      correctiveAction: "Restore a trusted schema-6 ZIP.",
    };
    await expect(useInternalProjectArchiveV6Session.getState().open(async () => ({
      status: "blocked",
      diagnostic,
    }))).resolves.toBe("blocked");
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      phase: "active",
      failure: diagnostic,
    });
    expect(useInternalProjectArchiveV6Session.getState().session?.snapshot).toBe(snapshot);
  });

  it("maps thrown reader failures into a closed fail-safe state", async () => {
    const error = Object.assign(new Error("Malformed native response."), {
      code: "schema6_archive_read.field_missing",
    });

    await expect(useInternalProjectArchiveV6Session.getState().open(async () => {
      throw error;
    })).resolves.toBe("blocked");

    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      phase: "error",
      session: null,
      failure: {
        code: "schema6_archive_read.field_missing",
        message: "Malformed native response.",
      },
    });
  });

  it("cannot be reactivated by a late native response after explicit close", async () => {
    const pending = deferred<{
      status: "ok";
      value: InternalProjectArchiveV6ReadSnapshotV1;
    }>();
    const opening = useInternalProjectArchiveV6Session.getState().open(() => pending.promise);
    expect(useInternalProjectArchiveV6Session.getState().phase).toBe("opening");

    useInternalProjectArchiveV6Session.getState().deactivate();
    pending.resolve({ status: "ok", value: snapshot });

    await expect(opening).resolves.toBe("stale");
    expect(useInternalProjectArchiveV6Session.getState()).toMatchObject({
      phase: "inactive",
      session: null,
    });
  });
});
