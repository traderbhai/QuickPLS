import { beforeEach, describe, expect, it, vi } from "vitest";
import { convertLegacyBasicModelV4, scientificSemModelV4HashInput, type SemModelV4 } from "./domain/semModelV4";
import { modelFingerprint } from "./domain/diagramGraph";
import { sha256HexUtf8V1 } from "./domain/sha256V1";
import { parseStandardSemModelV4AuthorityRecordV1 } from "./domain/standardSemModelV4Authority";
import type { StandardSemModelV4AuthorityCasOutcomeV1 } from "./domain/standardSemModelV4AuthorityCas";
import { useWorkspace } from "./store";

const compareAndSwap = vi.hoisted(() => vi.fn());
vi.mock("./services/standardSemModelV4AuthorityService", () => ({
  compareAndSwapStandardSemModelV4Authority: compareAndSwap,
}));

const DIGEST_A = "a".repeat(64);
const DIGEST_B = "b".repeat(64);

function strictAuthority(modelId: string) {
  return parseStandardSemModelV4AuthorityRecordV1({
    schema_version: 1,
    model_document_sha256: DIGEST_A,
    model: convertLegacyBasicModelV4({
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
    }, "pls_composite"),
  });
}

function successfulOutcome(
  expected: string,
  candidate: SemModelV4,
): StandardSemModelV4AuthorityCasOutcomeV1 {
  return {
    status: "ok",
    value: {
      schemaVersion: 1,
      sourceModelDocumentSha256: expected,
      canonicalCandidate: candidate,
      candidateModelDocumentSha256: DIGEST_B,
      candidateScientificSha256: null,
      readiness: "ready",
      authoringIssues: [],
      readinessIssues: [],
    },
  };
}

describe("workspace model edit command gateway v1", () => {
  beforeEach(() => {
    compareAndSwap.mockReset();
    useWorkspace.getState().resetProject();
  });

  it("applies one legacy scientific transaction with stable IDs and exact undo", async () => {
    const before = useWorkspace.getState();
    const construct = before.nodes.find((node) => !node.data.semantic)!;
    const ids = before.nodes.map((node) => node.id);
    const historyLength = before.past.length;

    await expect(before.executeModelEditCommand({
      kind: "rename_construct",
      constructId: construct.id,
      label: "Renamed construct",
    })).resolves.toMatchObject({
      status: "applied",
      transaction: "scientific",
      authority: "legacy_graph",
      affected: { constructIds: [construct.id] },
      undoable: true,
      stableIdsPreserved: true,
    });

    expect(useWorkspace.getState().nodes.map((node) => node.id)).toEqual(ids);
    expect(useWorkspace.getState().nodes.find((node) => node.id === construct.id)?.data.label).toBe("Renamed construct");
    expect(useWorkspace.getState().past).toHaveLength(historyLength + 1);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes.find((node) => node.id === construct.id)?.data.label).toBe(construct.data.label);

    const blockedHistory = useWorkspace.getState().past.length;
    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "rename_construct",
      constructId: construct.id,
      label: "  ",
    })).resolves.toMatchObject({
      status: "blocked",
      code: "model_edit.label_required",
      correctiveAction: "Enter a nonempty construct name.",
    });
    expect(useWorkspace.getState().past).toHaveLength(blockedHistory);
  });

  it("creates and moves one legacy construct through undoable gateway transactions", async () => {
    const state = useWorkspace.getState();
    const beforeIds = state.nodes.map((node) => node.id);
    await expect(state.executeModelEditCommand({
      kind: "add_construct",
      constructId: "construct:gateway",
      label: "Gateway construct",
      position: { x: 2_600, y: 1_600 },
    })).resolves.toMatchObject({ status: "applied", transaction: "scientific", affected: { constructIds: ["construct:gateway"] } });
    expect(useWorkspace.getState().nodes.find((node) => node.id === "construct:gateway")?.position).toEqual({ x: 2_600, y: 1_600 });

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "move_construct",
      constructId: "construct:gateway",
      position: { x: 2_720, y: 1_680 },
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation" });
    expect(useWorkspace.getState().nodes.find((node) => node.id === "construct:gateway")?.position).toEqual({ x: 2_720, y: 1_680 });
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes.find((node) => node.id === "construct:gateway")?.position).toEqual({ x: 2_600, y: 1_600 });
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes.map((node) => node.id)).toEqual(beforeIds);
  });

  it("keeps path route and label edits presentation-only", async () => {
    const edge = useWorkspace.getState().edges.find((candidate) => !candidate.id.startsWith("measurement::"))!;
    const currentRouting = useWorkspace.getState().diagramLayout.edgeLayouts[edge.id]?.routing ?? "straight";
    const changedRouting = currentRouting === "orthogonal" ? "curved" : "orthogonal";
    await expect(useWorkspace.getState().executeModelEditCommand({ kind: "set_path_routing", relationId: edge.id, routing: changedRouting })).resolves.toMatchObject({
      status: "applied",
      transaction: "presentation",
      affected: { relationshipIds: [edge.id] },
    });
    await expect(useWorkspace.getState().executeModelEditCommand({ kind: "nudge_path_label", relationId: edge.id, offset: { x: 9, y: -4 } })).resolves.toMatchObject({ status: "applied" });
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edge.id]).toMatchObject({
      routing: changedRouting,
      labelOffset: { x: 9, y: -4 },
      pinned: true,
    });
    await expect(useWorkspace.getState().executeModelEditCommand({ kind: "reset_path_route", relationId: edge.id })).resolves.toMatchObject({ status: "applied" });
    await expect(useWorkspace.getState().executeModelEditCommand({ kind: "reset_path_label", relationId: edge.id })).resolves.toMatchObject({ status: "applied" });
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edge.id]).toMatchObject({ routing: "straight", pinned: false });
  });

  it("persists editable polyline bends as one presentation transaction and restores them through undo", async () => {
    const edge = useWorkspace.getState().edges.find((candidate) => !candidate.id.startsWith("measurement::"))!;
    const before = structuredClone(useWorkspace.getState().diagramLayout.edgeLayouts[edge.id]);
    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "set_path_routing",
      relationId: edge.id,
      routing: "polyline",
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation" });
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edge.id]).toMatchObject({
      routing: "polyline",
      pinned: true,
      bendPoints: [expect.objectContaining({ x: expect.any(Number), y: expect.any(Number) })],
    });

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "set_path_bend_points",
      relationId: edge.id,
      points: [{ x: 320, y: 70 }, { x: 420, y: 170 }],
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation", affected: { relationshipIds: [edge.id] } });
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edge.id]?.bendPoints).toEqual([{ x: 320, y: 70 }, { x: 420, y: 170 }]);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edge.id]?.routing).toBe("polyline");
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edge.id]).toEqual(before);
  });

  it("stores measurement connector routes as presentation-only overrides with bulk reset and undo", async () => {
    const before = useWorkspace.getState();
    const construct = before.nodes.find((node) => node.data.indicators.length >= 2)!;
    const first = construct.data.indicators[0]!;
    const second = construct.data.indicators[1]!;
    const selection = { constructId: construct.id, indicator: first };
    const scientificBefore = structuredClone({ nodes: before.nodes, edges: before.edges });
    const fingerprintBefore = modelFingerprint(before.nodes, before.edges);
    before.setSelectedMeasurementConnector(selection);

    await expect(before.executeModelEditCommand({
      kind: "set_measurement_connector_routing",
      constructId: construct.id,
      routing: "curved",
    })).resolves.toMatchObject({
      status: "applied",
      transaction: "presentation",
      affected: { constructIds: [construct.id], indicatorIds: construct.data.indicators, relationshipIds: [] },
    });
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[construct.id]).toMatchObject({
      [first]: { routing: "curved" },
      [second]: { routing: "curved" },
    });
    expect(useWorkspace.getState().selectedMeasurementConnector).toEqual(selection);

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "set_measurement_connector_bend_points",
      constructId: construct.id,
      column: first,
      points: [{ x: 320, y: 70 }, { x: 420, y: 170 }],
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation" });
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[construct.id][first]).toEqual({
      routing: "polyline",
      bendPoints: [{ x: 320, y: 70 }, { x: 420, y: 170 }],
    });
    expect(modelFingerprint(useWorkspace.getState().nodes, useWorkspace.getState().edges)).toBe(fingerprintBefore);

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "set_measurement_connector_routing",
      constructId: construct.id,
      column: first,
      routing: "straight",
    })).resolves.toMatchObject({ status: "applied" });
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[construct.id][first]).toBeUndefined();
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[construct.id][second]).toEqual({ routing: "curved" });

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "reset_measurement_connector_route",
      constructId: construct.id,
    })).resolves.toMatchObject({ status: "applied" });
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[construct.id]).toBeUndefined();
    expect({ nodes: useWorkspace.getState().nodes, edges: useWorkspace.getState().edges }).toEqual(scientificBefore);
    expect(useWorkspace.getState().selectedMeasurementConnector).toEqual(selection);

    useWorkspace.getState().undo();
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[construct.id][second]).toEqual({ routing: "curved" });
  });

  it("rejects invalid measurement connector targets and bend points without creating history", async () => {
    const state = useWorkspace.getState();
    const construct = state.nodes.find((node) => node.data.indicators.length)!;
    const historyLength = state.past.length;

    await expect(state.executeModelEditCommand({
      kind: "set_measurement_connector_routing",
      constructId: construct.id,
      column: "not-assigned",
      routing: "orthogonal",
    })).resolves.toMatchObject({ status: "blocked", code: "model_edit.indicator_not_assigned" });
    await expect(state.executeModelEditCommand({
      kind: "set_measurement_connector_bend_points",
      constructId: construct.id,
      column: construct.data.indicators[0]!,
      points: [],
    })).resolves.toMatchObject({ status: "blocked", code: "model_edit.measurement_connector_bends_invalid" });
    expect(useWorkspace.getState().past).toHaveLength(historyLength);
  });

  it("keeps measurement connector selection mutually exclusive with construct and path selection", () => {
    const state = useWorkspace.getState();
    const construct = state.nodes.find((node) => node.data.indicators.length)!;
    const selection = { constructId: construct.id, indicator: construct.data.indicators[0]! };
    state.setSelectedMeasurementConnector(selection);
    expect(useWorkspace.getState()).toMatchObject({
      selectedMeasurementConnector: selection,
      selectedNodeId: null,
      selectedEdgeId: null,
    });

    useWorkspace.getState().setSelectedNode(construct.id);
    expect(useWorkspace.getState().selectedMeasurementConnector).toBeNull();
    useWorkspace.getState().setSelectedMeasurementConnector(selection);
    useWorkspace.getState().setSelectedEdge(useWorkspace.getState().edges[0]!.id);
    expect(useWorkspace.getState().selectedMeasurementConnector).toBeNull();
  });

  it("stores moderation anchor movement only in presentation metadata", async () => {
    const state = useWorkspace.getState();
    const focal = state.edges.find((edge) => !edge.id.startsWith("measurement::") && edge.data?.role !== "covariance")!;
    const predictor = state.nodes.find((node) => node.id === focal.source)!;
    const outcome = state.nodes.find((node) => node.id === focal.target)!;
    const moderator = {
      ...predictor,
      id: "moderator:anchor-test",
      position: { x: predictor.position.x, y: predictor.position.y + 400 },
      data: { ...predictor.data, label: "Moderator", shortName: "W" },
    };
    const interaction = {
      ...predictor,
      id: "interaction:anchor-test",
      position: { x: 0, y: 0 },
      data: {
        ...predictor.data,
        label: "X × W",
        shortName: "XW",
        indicators: [],
        semantic: "interaction" as const,
        interaction: {
          kind: "interaction_v2" as const,
          termId: "term:anchor-test",
          operands: [predictor.id, moderator.id] as [string, string],
          focalRelationId: focal.id,
          outcome: outcome.id,
          canonicalMethod: "two_stage" as const,
          hierarchyPolicy: "strong" as const,
        },
      },
    };
    useWorkspace.setState({ nodes: [...state.nodes, moderator, interaction] });
    const scientificBefore = structuredClone({ nodes: useWorkspace.getState().nodes, edges: useWorkspace.getState().edges });

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "set_moderation_anchor_fraction",
      interactionTermId: "term:anchor-test",
      fraction: 0.01,
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation", affected: { relationshipIds: [focal.id] } });
    expect(useWorkspace.getState().diagramLayout.moderationAnchorFractions).toMatchObject({ "term:anchor-test": 0.2 });
    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "set_moderation_anchor_fraction",
      interactionTermId: "term:anchor-test",
      fraction: 0.99,
    })).resolves.toMatchObject({ status: "applied", transaction: "presentation", affected: { relationshipIds: [focal.id] } });
    expect(useWorkspace.getState().diagramLayout.moderationAnchorFractions).toMatchObject({ "term:anchor-test": 0.8 });
    expect({ nodes: useWorkspace.getState().nodes, edges: useWorkspace.getState().edges }).toEqual(scientificBefore);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().diagramLayout.moderationAnchorFractions).toMatchObject({ "term:anchor-test": 0.2 });
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().diagramLayout.moderationAnchorFractions?.["term:anchor-test"]).toBeUndefined();
  });

  it("moves legacy indicators atomically and keeps unrelated presentation metadata", async () => {
    const initial = useWorkspace.getState();
    const target = initial.nodes[0]!;
    const source = initial.nodes.find((node) => node.id !== target.id && node.data.indicators.length)!;
    const column = source.data.indicators[0]!;
    const unrelatedEdge = initial.edges[0]!;
    await initial.executeModelEditCommand({
      kind: "set_measurement_connector_routing",
      constructId: source.id,
      column,
      routing: "curved",
    });
    initial.setSelectedMeasurementConnector({ constructId: source.id, indicator: column });
    initial.nudgeEdgeLabel(unrelatedEdge.id, { x: 14, y: -6 });
    const priorEdgeLayout = structuredClone(useWorkspace.getState().diagramLayout.edgeLayouts[unrelatedEdge.id]);

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "assign_indicators",
      constructId: target.id,
      columns: [column],
    })).resolves.toMatchObject({ status: "applied", transaction: "scientific" });

    expect(useWorkspace.getState().nodes.find((node) => node.id === target.id)?.data.indicators).toContain(column);
    expect(useWorkspace.getState().nodes.find((node) => node.id === source.id)?.data.indicators).not.toContain(column);
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[unrelatedEdge.id]).toEqual(priorEdgeLayout);
    expect(useWorkspace.getState().selectedMeasurementConnector).toBeNull();
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[source.id]?.[column]).toBeUndefined();
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes.find((node) => node.id === source.id)?.data.indicators).toContain(column);
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[source.id]?.[column]).toEqual({ routing: "curved" });
  });

  it("clears connector selection and prunes orphan routes in legacy assignment helpers", async () => {
    const initial = useWorkspace.getState();
    const target = initial.nodes.find((node) => !node.data.semantic)!;
    const source = initial.nodes.find((node) => node.id !== target.id && node.data.indicators.length >= 2)!;
    const [singleColumn, bulkColumn] = source.data.indicators;

    await initial.executeModelEditCommand({
      kind: "set_measurement_connector_routing",
      constructId: source.id,
      column: singleColumn!,
      routing: "orthogonal",
    });
    useWorkspace.getState().setSelectedMeasurementConnector({ constructId: source.id, indicator: singleColumn! });
    useWorkspace.getState().assignIndicator(target.id, singleColumn!);
    expect(useWorkspace.getState().selectedMeasurementConnector).toBeNull();
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[source.id]?.[singleColumn!]).toBeUndefined();

    await useWorkspace.getState().executeModelEditCommand({
      kind: "set_measurement_connector_routing",
      constructId: source.id,
      column: bulkColumn!,
      routing: "curved",
    });
    useWorkspace.getState().setSelectedMeasurementConnector({ constructId: source.id, indicator: bulkColumn! });
    useWorkspace.getState().assignIndicators(target.id, [bulkColumn!]);
    expect(useWorkspace.getState().selectedMeasurementConnector).toBeNull();
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[source.id]?.[bulkColumn!]).toBeUndefined();

    await useWorkspace.getState().executeModelEditCommand({
      kind: "set_measurement_connector_routing",
      constructId: target.id,
      column: bulkColumn!,
      routing: "polyline",
    });
    useWorkspace.getState().setSelectedMeasurementConnector({ constructId: target.id, indicator: bulkColumn! });
    useWorkspace.getState().unassignIndicator(target.id, bulkColumn!);
    expect(useWorkspace.getState().selectedMeasurementConnector).toBeNull();
    expect(useWorkspace.getState().diagramLayout.measurementConnectorLayouts[target.id]?.[bulkColumn!]).toBeUndefined();
  });

  it("preserves pinned constructs and every unrelated manual override during Arrange", async () => {
    const first = useWorkspace.getState().nodes[0]!;
    const second = useWorkspace.getState().nodes[1]!;
    const edge = useWorkspace.getState().edges[0]!;
    await useWorkspace.getState().executeModelEditCommand({ kind: "set_construct_pinned", constructId: first.id, pinned: true });
    await useWorkspace.getState().executeModelEditCommand({ kind: "set_construct_indicator_side", constructId: first.id, side: "right" });
    useWorkspace.getState().nudgeEdgeLabel(edge.id, { x: 11, y: -9 });
    useWorkspace.setState((state) => {
      const nodes = state.nodes.map((node) => node.id === second.id ? { ...node, position: { x: 1_400, y: 900 } } : node);
      return {
        nodes,
        diagramLayout: {
          ...state.diagramLayout,
          constructLayouts: {
            ...state.diagramLayout.constructLayouts,
            [second.id]: { ...state.diagramLayout.constructLayouts[second.id], x: 1_400, y: 900 },
          },
          moderationAnchorFractions: { "term:test": 0.45 },
          moderationConnectorBendPoints: { "connector:test": [{ x: 21, y: 34 }] },
        },
      };
    });
    const beforeArrange = useWorkspace.getState();
    const pinnedPosition = { ...beforeArrange.nodes.find((node) => node.id === first.id)!.position };
    const indicatorLayout = structuredClone(beforeArrange.diagramLayout.indicatorLayouts[first.id]);
    const edgeLayout = structuredClone(beforeArrange.diagramLayout.edgeLayouts[edge.id]);
    const completeLayout = structuredClone(beforeArrange.diagramLayout);
    const completeNodes = structuredClone(beforeArrange.nodes);

    await expect(beforeArrange.executeModelEditCommand({ kind: "arrange_model", direction: "horizontal" })).resolves.toMatchObject({
      status: "applied",
      transaction: "presentation",
      stableIdsPreserved: true,
    });
    const arranged = useWorkspace.getState();
    expect(arranged.nodes.find((node) => node.id === first.id)?.position).toEqual(pinnedPosition);
    expect(arranged.diagramLayout.constructLayouts[first.id].pinned).toBe(true);
    expect(arranged.diagramLayout.indicatorLayouts[first.id]).toEqual(indicatorLayout);
    expect(arranged.diagramLayout.edgeLayouts[edge.id]).toEqual(edgeLayout);
    expect(arranged.diagramLayout.moderationAnchorFractions).toEqual({ "term:test": 0.45 });
    expect(arranged.diagramLayout.moderationConnectorBendPoints).toEqual({ "connector:test": [{ x: 21, y: 34 }] });

    arranged.undo();
    expect(useWorkspace.getState().nodes).toEqual(completeNodes);
    expect(useWorkspace.getState().diagramLayout).toEqual(completeLayout);
  });

  it("routes strict scientific edits through CAS and maps revision locks to an explicit correction", async () => {
    const modelId = useWorkspace.getState().activeModelId!;
    const authority = strictAuthority(modelId);
    const predictorId = authority.model.variables.find((variable) => variable.label === "Predictor")?.id;
    if (!predictorId) throw new Error("Expected the resident Predictor identity.");
    expect(useWorkspace.getState().installStandardSemModelV4Authority(authority)).toBe(true);
    await useWorkspace.getState().executeModelEditCommand({ kind: "set_construct_pinned", constructId: predictorId, pinned: true });
    const predictorIndicator = useWorkspace.getState().nodes.find((node) => node.id === predictorId)!.data.indicators[0]!;
    useWorkspace.getState().setSelectedMeasurementConnector({ constructId: predictorId, indicator: predictorIndicator });
    const layoutBefore = structuredClone(useWorkspace.getState().diagramLayout);
    compareAndSwap.mockImplementation(async (_source: SemModelV4, expected: string, candidate: SemModelV4) => successfulOutcome(expected, candidate));

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "rename_construct",
      constructId: predictorId,
      label: "Strict predictor renamed",
    })).resolves.toMatchObject({
      status: "applied",
      transaction: "scientific",
      authority: "standard_sem_model_v4",
      affected: { constructIds: [predictorId] },
    });
    expect(compareAndSwap).toHaveBeenCalledTimes(1);
    expect(useWorkspace.getState().selectedMeasurementConnector).toBeNull();
    expect(useWorkspace.getState().standardSemModelV4Authorities[modelId].model.variables.find((variable) => variable.id === predictorId)?.label).toBe("Strict predictor renamed");
    expect(useWorkspace.getState().diagramLayout.constructLayouts[predictorId]).toEqual(layoutBefore.constructLayouts[predictorId]);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().standardSemModelV4Authorities[modelId].model.variables.find((variable) => variable.id === predictorId)?.label).toBe("Predictor");

    useWorkspace.setState((state) => ({
      standardSemModelV4ScientificEditLocks: { ...state.standardSemModelV4ScientificEditLocks, [modelId]: true },
    }));
    compareAndSwap.mockClear();
    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "rename_construct",
      constructId: predictorId,
      label: "Blocked rename",
    })).resolves.toMatchObject({
      status: "blocked",
      code: "schema6_standard_authority.scientific_revision_fork_required",
      correctiveAction: expect.stringContaining("new revision"),
    });
    expect(compareAndSwap).not.toHaveBeenCalled();
  });

  it("applies measurement connector routing without changing the strict scientific digest", async () => {
    const modelId = useWorkspace.getState().activeModelId!;
    const authority = strictAuthority(modelId);
    expect(useWorkspace.getState().installStandardSemModelV4Authority(authority)).toBe(true);
    const construct = useWorkspace.getState().nodes.find((node) => node.data.indicators.length)!;
    const indicator = construct.data.indicators[0]!;
    const selection = { constructId: construct.id, indicator };
    const scientificDigestBefore = sha256HexUtf8V1(scientificSemModelV4HashInput(authority.model));
    useWorkspace.getState().setSelectedMeasurementConnector(selection);

    await expect(useWorkspace.getState().executeModelEditCommand({
      kind: "set_measurement_connector_routing",
      constructId: construct.id,
      column: indicator,
      routing: "orthogonal",
    })).resolves.toMatchObject({
      status: "applied",
      transaction: "presentation",
      authority: "standard_sem_model_v4",
      affected: { constructIds: [construct.id], indicatorIds: [indicator], relationshipIds: [] },
    });
    expect(compareAndSwap).not.toHaveBeenCalled();
    expect(useWorkspace.getState().selectedMeasurementConnector).toEqual(selection);
    expect(useWorkspace.getState().standardSemModelV4Layouts[modelId].diagram_layout.measurementConnectorLayouts)
      .toMatchObject({ [construct.id]: { [indicator]: { routing: "orthogonal" } } });
    const retainedAuthority = useWorkspace.getState().standardSemModelV4Authorities[modelId];
    expect(retainedAuthority.model_document_sha256).toBe(authority.model_document_sha256);
    expect(sha256HexUtf8V1(scientificSemModelV4HashInput(retainedAuthority.model))).toBe(scientificDigestBefore);
  });
});
