import { beforeEach, describe, expect, it, vi } from "vitest";
import { convertLegacyBasicModelV4, type SemModelV4 } from "./domain/semModelV4";
import { parseStandardSemModelV4AuthorityRecordV1 } from "./domain/standardSemModelV4Authority";
import type { StandardSemModelV4AuthorityCasOutcomeV1 } from "./domain/standardSemModelV4AuthorityCas";
import { useWorkspace } from "./store";

const compareAndSwap = vi.hoisted(() => vi.fn());
vi.mock("./services/standardSemModelV4AuthorityService", () => ({
  compareAndSwapStandardSemModelV4Authority: compareAndSwap,
}));

const DIGEST_A = "a".repeat(64);
const DIGEST_B = "b".repeat(64);
const DIGEST_C = "c".repeat(64);

function authorityFor(modelId: string, digest = DIGEST_A) {
  const model = convertLegacyBasicModelV4({
    id: modelId,
    name: "Strict authority",
    constructs: [
      { id: "x", name: "Predictor", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
      { id: "y", name: "Outcome", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
    ],
    paths: [{ source: "x", target: "y" }],
    controls: [],
    higher_order_constructs: [],
    interactions: [],
  }, "pls_composite");
  return parseStandardSemModelV4AuthorityRecordV1({
    schema_version: 1,
    model_document_sha256: digest,
    model,
  });
}

function successfulOutcome(
  sourceDigest: string,
  candidateDigest: string,
  candidate: SemModelV4,
): StandardSemModelV4AuthorityCasOutcomeV1 {
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      sourceModelDocumentSha256: sourceDigest,
      canonicalCandidate: candidate,
      candidateModelDocumentSha256: candidateDigest,
      candidateScientificSha256: null,
      readiness: "ready",
      authoringIssues: [],
      readinessIssues: [],
    },
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise;
  });
  return { promise, resolve };
}

function installActiveAuthority() {
  const state = useWorkspace.getState();
  const modelId = state.activeModelId!;
  const authority = authorityFor(modelId);
  expect(state.installStandardSemModelV4Authority(authority)).toBe(true);
  return { modelId, authority: useWorkspace.getState().standardSemModelV4Authorities[modelId] };
}

describe("Standard SemModelV4 store authority", () => {
  beforeEach(() => {
    compareAndSwap.mockReset();
    useWorkspace.getState().resetProject();
  });

  it("keeps graph values projection-only and exposes hard blockers for legacy consumers", () => {
    const { modelId, authority } = installActiveAuthority();
    const installed = useWorkspace.getState();
    const projectedNodes = installed.nodes;
    const projectedEdges = installed.edges;

    for (const operation of ["schema5_save", "schema5_autosave", "calculation", "legacy_graph_serialization"] as const) {
      expect(installed.standardSemModelV4OperationBlocker(operation)).toContain(modelId);
    }
    expect(installed.modelPresentations[modelId]).toBeUndefined();

    installed.addConstruct({ x: 10, y: 20 });
    installed.updateConstruct(projectedNodes[0].id, { label: "Graph-only edit" });
    installed.addPath(projectedNodes[0].id, projectedNodes.at(-1)!.id);
    installed.onNodesChange([{ id: projectedNodes[0].id, type: "remove" }]);
    const blocked = useWorkspace.getState();
    expect(blocked.standardSemModelV4Authorities[modelId]).toBe(authority);
    expect(blocked.nodes).toBe(projectedNodes);
    expect(blocked.edges).toBe(projectedEdges);

    blocked.onNodesChange([{
      id: projectedNodes[0].id,
      type: "position",
      position: { x: 321, y: 654 },
    }]);
    const laidOut = useWorkspace.getState();
    expect(laidOut.nodes.find((node) => node.id === projectedNodes[0].id)?.position).toEqual({ x: 321, y: 654 });
    expect(laidOut.standardSemModelV4Authorities[modelId]).toBe(authority);

    laidOut.setProjectExplorer({
      projectModels: [],
      activeModelId: modelId,
      modelPresentations: {},
      savedReports: [],
    });
    const removed = useWorkspace.getState();
    expect(removed.activeModelId).toBeNull();
    expect(removed.standardSemModelV4Authorities[modelId]).toBeUndefined();
    expect(removed.nodes).toEqual([]);
    expect(removed.edges).toEqual([]);

    useWorkspace.getState().resetProject();
    const legacy = useWorkspace.getState();
    expect(legacy.standardSemModelV4OperationBlocker("schema5_save")).toBeNull();
    const legacyCount = legacy.nodes.length;
    legacy.addConstruct({ x: 10, y: 20 });
    expect(useWorkspace.getState().nodes).toHaveLength(legacyCount + 1);
  });

  it("activates descriptor-only schema-6 data and keeps multiple strict models switchable without legacy science", () => {
    const first = authorityFor("strict-one");
    const secondBase = authorityFor("strict-two", DIGEST_B);
    const second = parseStandardSemModelV4AuthorityRecordV1({
      ...secondBase,
      model: { ...secondBase.model, name: "Second strict model" },
    });
    const descriptor = {
      id: "legacy-unbound",
      name: "Schema-6 descriptor",
      columns: ["x1", "x2", "y1", "y2"],
      columnMetadata: ["x1", "x2", "y1", "y2"].map((name) => ({
        name,
        label: null,
        column_type: "numeric" as const,
        scale_type: "continuous" as const,
        missing_markers: ["NA"],
        theoretical_min: null,
        theoretical_max: null,
        value_labels: {},
      })),
      rowCount: 321,
      fingerprint: "dataset-fingerprint",
      kind: "raw" as const,
      sampleSize: null,
    };
    expect(useWorkspace.getState().activateStandardSemModelV4Authorities([
      { authority: first, readiness: "ready", scientificSha256: "d".repeat(64) },
      { authority: second, readiness: "authoring_only", scientificSha256: null },
    ], first.model.id, "Schema-6 project", [descriptor])).toBe(true);

    const activated = useWorkspace.getState();
    expect(activated.projectModels).toEqual([]);
    expect(activated.datasetDescriptorOnly).toBe(true);
    expect(activated.dataset).toMatchObject({
      id: descriptor.id,
      name: descriptor.name,
      columns: descriptor.columns,
      rows: [],
      rowCount: 321,
      fingerprint: descriptor.fingerprint,
      kind: "raw",
      sampleSize: null,
      columnMetadata: descriptor.columnMetadata,
    });
    expect(Number.isNaN(activated.dataset.missing)).toBe(true);
    expect(Object.values(activated.standardSemModelV4Authorities).map((authority) => [
      authority.model.id,
      authority.model.name,
    ])).toEqual([
      ["strict-one", "Strict authority"],
      ["strict-two", "Second strict model"],
    ]);
    expect(activated.switchProjectModel("strict-two")).toBe(true);
    expect(useWorkspace.getState().activeModelId).toBe("strict-two");

    const beforeBlockedDataEdit = useWorkspace.getState();
    beforeBlockedDataEdit.setDataset({
      id: "invented",
      name: "Invented rows",
      columns: ["z"],
      rows: [{ z: 1 }],
      missing: 0,
    });
    expect(useWorkspace.getState()).toBe(beforeBlockedDataEdit);
    expect(useWorkspace.getState().standardSemModelV4OperationBlocker("calculation")).toContain("strict-two");
  });

  it("locks a bound source, then appends an unlocked new-identity authority", async () => {
    const source = authorityFor("recipe-bound-source");
    const revision = authorityFor("model:revision:new", DIGEST_B);
    const descriptor = {
      id: "legacy-unbound",
      name: "Descriptor",
      columns: ["x1", "x2", "y1", "y2"],
      columnMetadata: ["x1", "x2", "y1", "y2"].map((name) => ({
        name, label: null, column_type: "numeric" as const, scale_type: "continuous" as const,
        missing_markers: [], theoretical_min: null, theoretical_max: null, value_labels: {},
      })),
      rowCount: 50,
      fingerprint: "dataset-fingerprint",
      kind: "raw" as const,
      sampleSize: null,
    };
    expect(useWorkspace.getState().activateStandardSemModelV4Authorities([
      { authority: source, readiness: "ready", scientificSha256: "d".repeat(64) },
    ], source.model.id, "Schema-6", [descriptor], [source.model.id])).toBe(true);
    const before = useWorkspace.getState();
    const sourceEpoch = before.standardSemModelV4Epochs[source.model.id];
    await expect(before.commitStandardSemModelV4Intent({ kind: "set_model_name", name: "Forbidden rewrite" }))
      .resolves.toMatchObject({
        status: "blocked",
        diagnostic: { code: "schema6_standard_authority.scientific_revision_fork_required" },
      });
    expect(compareAndSwap).not.toHaveBeenCalled();

    expect(before.appendStandardSemModelV4Revision({
      sourceModelId: source.model.id,
      expectedSourceModelDocumentSha256: source.model_document_sha256,
      expectedSourceEpoch: sourceEpoch,
    }, {
      authority: revision,
      readiness: "ready",
      scientificSha256: "e".repeat(64),
    })).toBe(true);

    const appended = useWorkspace.getState();
    expect(appended.standardSemModelV4Authorities[source.model.id]).toEqual(source);
    expect(appended.standardSemModelV4Authorities[revision.model.id]).toEqual(revision);
    expect(appended.activeModelId).toBe(revision.model.id);
    expect(appended.explorerSelection).toEqual({ kind: "model", modelId: revision.model.id });
    expect(appended.standardSemModelV4ScientificEditLocks).toEqual({ [source.model.id]: true });
    expect(appended.captureStandardSemModelV4SaveAuthorities([source.model.id, revision.model.id]))
      .toMatchObject({
        [source.model.id]: { dirty: false },
        [revision.model.id]: { dirty: false },
      });
    expect(appended.appendStandardSemModelV4Revision({
      sourceModelId: source.model.id,
      expectedSourceModelDocumentSha256: source.model_document_sha256,
      expectedSourceEpoch: sourceEpoch,
    }, { authority: authorityFor("model:revision:stale"), readiness: "ready", scientificSha256: "f".repeat(64) }))
      .toBe(false);

    compareAndSwap.mockResolvedValue(successfulOutcome(DIGEST_B, DIGEST_C, {
      ...revision.model,
      name: "Editable revision",
    }));
    await expect(useWorkspace.getState().commitStandardSemModelV4Intent({
      kind: "set_model_name",
      name: "Editable revision",
    })).resolves.toMatchObject({ status: "committed" });
  });

  it("serializes per-model commits FIFO and advances each CAS from the committed document digest", async () => {
    const { modelId } = installActiveAuthority();
    const firstGate = deferred<void>();
    const calls: Array<{ expected: string; sourceName: string; candidateName: string }> = [];
    compareAndSwap.mockImplementation(async (source: SemModelV4, expected: string, candidate: SemModelV4) => {
      calls.push({ expected, sourceName: source.name, candidateName: candidate.name });
      if (calls.length === 1) await firstGate.promise;
      return successfulOutcome(expected, calls.length === 1 ? DIGEST_B : DIGEST_C, candidate);
    });

    const first = useWorkspace.getState().commitStandardSemModelV4Intent(
      { kind: "set_model_name", name: "First committed name" },
    );
    const second = useWorkspace.getState().commitStandardSemModelV4Intent(
      { kind: "set_model_name", name: "Second committed name" },
    );
    await vi.waitFor(() => expect(compareAndSwap).toHaveBeenCalledTimes(1));
    firstGate.resolve(undefined);

    await expect(first).resolves.toMatchObject({ status: "committed" });
    await expect(second).resolves.toMatchObject({ status: "committed" });
    expect(calls).toEqual([
      { expected: DIGEST_A, sourceName: "Strict authority", candidateName: "First committed name" },
      { expected: DIGEST_B, sourceName: "First committed name", candidateName: "Second committed name" },
    ]);
    expect(useWorkspace.getState().standardSemModelV4Authorities[modelId]).toMatchObject({
      model_document_sha256: DIGEST_C,
      model: { name: "Second committed name" },
    });

    useWorkspace.getState().undo();
    expect(useWorkspace.getState().standardSemModelV4Authorities[modelId]).toMatchObject({
      model_document_sha256: DIGEST_B,
      model: { name: "First committed name" },
    });
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().standardSemModelV4Authorities[modelId]).toMatchObject({
      model_document_sha256: DIGEST_A,
      model: { name: "Strict authority" },
    });
    useWorkspace.getState().redo();
    useWorkspace.getState().redo();
    expect(useWorkspace.getState().standardSemModelV4Authorities[modelId]).toMatchObject({
      model_document_sha256: DIGEST_C,
      model: { name: "Second committed name" },
    });
  });

  it("marks a parameter-only authority edit dirty even when its graph projection is unchanged, then reanchors it", async () => {
    const { modelId, authority } = installActiveAuthority();
    const parameter = authority.model.parameters.find((candidate) => candidate.kind === "free");
    expect(parameter).toBeDefined();
    const clean = useWorkspace.getState().captureStandardSemModelV4SaveAuthorities([modelId]);
    expect(clean?.[modelId].dirty).toBe(false);
    const projectedBefore = {
      nodes: useWorkspace.getState().nodes,
      edges: useWorkspace.getState().edges,
    };
    compareAndSwap.mockImplementation(async (_source: SemModelV4, expected: string, candidate: SemModelV4) =>
      successfulOutcome(expected, DIGEST_B, candidate));

    await expect(useWorkspace.getState().commitStandardSemModelV4Intent({
      kind: "set_parameter_specification",
      parameter_id: parameter!.id,
      specification: { kind: "free", start: 0.321, lower: null, upper: null, equality_label: null },
    })).resolves.toMatchObject({ status: "committed" });

    expect(useWorkspace.getState().nodes).toEqual(projectedBefore.nodes);
    expect(useWorkspace.getState().edges).toEqual(projectedBefore.edges);
    const dirty = useWorkspace.getState().captureStandardSemModelV4SaveAuthorities([modelId]);
    expect(dirty?.[modelId]).toMatchObject({ dirty: true, authority: { model_document_sha256: DIGEST_B } });
    expect(useWorkspace.getState().clearStandardSemModelV4Workspace([modelId])).toBe(false);
    expect(useWorkspace.getState().standardSemModelV4Authorities).toHaveProperty(modelId);
    expect(useWorkspace.getState().reanchorStandardSemModelV4Authorities(dirty!)).toBe(true);
    expect(useWorkspace.getState().captureStandardSemModelV4SaveAuthorities([modelId])?.[modelId].dirty).toBe(false);
    expect(useWorkspace.getState().clearStandardSemModelV4Workspace(["wrong-model"])).toBe(false);
    expect(useWorkspace.getState().clearStandardSemModelV4Workspace([modelId])).toBe(true);
    expect(useWorkspace.getState()).toMatchObject({
      activeModelId: null,
      standardSemModelV4Authorities: {},
      standardSemModelV4DatasetDescriptors: {},
      datasetDescriptorOnly: false,
      view: "welcome",
    });
  });

  it("leaves authority and projection unchanged on native blocking or rejection", async () => {
    installActiveAuthority();
    const beforeBlocked = useWorkspace.getState();
    compareAndSwap.mockResolvedValueOnce({
      status: "blocked",
      diagnostic: {
        code: "digest_mismatch",
        message: "The source changed.",
        correctiveAction: "Refresh and retry.",
        authoringIssues: [],
        readinessIssues: [],
      },
    });
    const blocked = await beforeBlocked.commitStandardSemModelV4Intent(
      { kind: "set_model_name", name: "Blocked name" },
    );
    expect(blocked).toMatchObject({ status: "blocked", diagnostic: { code: "digest_mismatch" } });
    expect(useWorkspace.getState()).toBe(beforeBlocked);

    const beforeRejected = useWorkspace.getState();
    compareAndSwap.mockRejectedValueOnce(new Error("native unavailable"));
    const rejected = await beforeRejected.commitStandardSemModelV4Intent(
      { kind: "set_model_name", name: "Rejected name" },
    );
    expect(rejected).toMatchObject({ status: "rejected" });
    expect(useWorkspace.getState()).toBe(beforeRejected);
  });

  it("invalidates a late CAS on model switch and never serializes a strict projection as legacy science", async () => {
    const initial = useWorkspace.getState();
    const firstModel = initial.projectModels[0];
    const secondModel = { ...firstModel, id: "legacy-second", name: "Legacy second" };
    initial.loadProject({
      nodes: initial.nodes,
      edges: initial.edges,
      dataset: initial.dataset,
      projectModels: [firstModel, secondModel],
      activeModelId: firstModel.id,
      modelPresentations: {
        [firstModel.id]: initial.modelPresentations[firstModel.id],
        [secondModel.id]: initial.modelPresentations[firstModel.id],
      },
    });
    const { authority } = installActiveAuthority();
    const strictLegacyMetadata = useWorkspace.getState().projectModels.find((model) => model.id === firstModel.id);
    const response = deferred<StandardSemModelV4AuthorityCasOutcomeV1>();
    compareAndSwap.mockImplementation(async () => response.promise);
    const pending = useWorkspace.getState().commitStandardSemModelV4Intent(
      { kind: "set_model_name", name: "Late name" },
    );
    await vi.waitFor(() => expect(compareAndSwap).toHaveBeenCalledTimes(1));

    expect(useWorkspace.getState().switchProjectModel(secondModel.id)).toBe(true);
    const switched = useWorkspace.getState();
    expect(switched.projectModels.find((model) => model.id === firstModel.id)).toEqual(strictLegacyMetadata);
    response.resolve(successfulOutcome(DIGEST_A, DIGEST_B, {
      ...authority.model,
      name: "Late name",
    }));

    await expect(pending).resolves.toEqual({ status: "stale" });
    expect(useWorkspace.getState()).toBe(switched);
    expect(useWorkspace.getState().standardSemModelV4Authorities[firstModel.id]).toBe(authority);

    useWorkspace.setState({ nodes: [], edges: [] });
    expect(useWorkspace.getState().switchProjectModel(firstModel.id)).toBe(true);
    expect(useWorkspace.getState().nodes.length).toBeGreaterThan(0);
    expect(useWorkspace.getState().standardSemModelV4Authorities[firstModel.id]).toBe(authority);
  });
});
