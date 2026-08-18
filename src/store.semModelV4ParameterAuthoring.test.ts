import { beforeEach, describe, expect, it } from "vitest";
import {
  nativeSemLatentMeanEntryV4,
  withNativeSemFactorIdentificationV4,
  withNativeSemParameterEntriesOnConstructV4,
  withNativeSemParameterEntryOnConstructV4,
  withNativeSemParameterEntryOnEdgeV4,
} from "./domain/semModelV4ParameterAuthoring";
import { useWorkspace } from "./store";
import type { PathEdgeData, SemModelV4ParameterAuthoringEntry } from "./types";

describe("SemModelV4 parameter metadata in the live store", () => {
  beforeEach(() => useWorkspace.getState().resetProject());

  it("applies a factor identification and latent mean atomically with deterministic undo and redo", () => {
    useWorkspace.getState().setConstructEstimandV4("competence", { kind: "common_factor", marker_indicator: "COMP1" });
    const authored = useWorkspace.getState().nodes.find((node) => node.id === "competence")!;
    const identified = withNativeSemFactorIdentificationV4(authored, { kind: "effects_coding" });
    const next = withNativeSemParameterEntriesOnConstructV4(identified, [nativeSemLatentMeanEntryV4("competence")]);

    useWorkspace.getState().updateConstruct("competence", { semModelV4: next.data.semModelV4 });
    let current = useWorkspace.getState().nodes.find((node) => node.id === "competence")!;
    expect(current.data.semModelV4).toMatchObject({
      identification: { kind: "effects_coding" },
      parameters: [expect.objectContaining({ parameter_id: "native-sem-v4:mean:competence" })],
    });

    useWorkspace.getState().undo();
    current = useWorkspace.getState().nodes.find((node) => node.id === "competence")!;
    expect(current.data.semModelV4?.identification).toMatchObject({ kind: "marker_loading", indicator: "COMP1" });
    expect(current.data.semModelV4?.parameters).toBeUndefined();

    useWorkspace.getState().redo();
    current = useWorkspace.getState().nodes.find((node) => node.id === "competence")!;
    expect(current.data.semModelV4).toEqual(next.data.semModelV4);
  });

  it("preserves stable construct and relationship parameter IDs through undo and JSON reopen", () => {
    useWorkspace.getState().setConstructEstimandV4("competence", { kind: "common_factor", marker_indicator: "COMP1" });
    const loadingEntry: SemModelV4ParameterAuthoringEntry = {
      parameter_id: "adapter:measurement_parameter:competence:COMP2",
      target: { kind: "loading", construct: "construct:competence", indicator: "observed:COMP2" },
      specification: { kind: "free", start: 0.7, lower: 0, upper: 1, equality_label: "loading_a" },
    };
    const node = useWorkspace.getState().nodes.find((candidate) => candidate.id === "competence")!;
    const nextNode = withNativeSemParameterEntryOnConstructV4(node, loadingEntry);
    useWorkspace.getState().updateConstruct(node.id, { semModelV4: nextNode.data.semModelV4 });

    const edge = useWorkspace.getState().edges.find((candidate) => candidate.id === "comp-cusa")!;
    const pathEntry: SemModelV4ParameterAuthoringEntry = {
      parameter_id: "adapter:structural_parameter:comp-cusa",
      target: { kind: "regression", source: `construct:${edge.source}`, target: `construct:${edge.target}` },
      specification: { kind: "fixed", value: 0.2 },
    };
    const nextEdge = withNativeSemParameterEntryOnEdgeV4(edge, pathEntry);
    useWorkspace.getState().updateEdge(edge.id, { data: nextEdge.data });
    expect(edgeAuthoring(useWorkspace.getState().edges.find((candidate) => candidate.id === edge.id))?.parameters[0]?.parameter_id).toBe(pathEntry.parameter_id);

    useWorkspace.getState().undo();
    expect(edgeAuthoring(useWorkspace.getState().edges.find((candidate) => candidate.id === edge.id))).toBeUndefined();
    useWorkspace.getState().redo();

    const saved = useWorkspace.getState();
    const nodes = JSON.parse(JSON.stringify(saved.nodes));
    const edges = JSON.parse(JSON.stringify(saved.edges));
    saved.loadProject({ nodes, edges, dataset: saved.dataset });
    const reopened = useWorkspace.getState();
    expect(reopened.nodes.find((candidate) => candidate.id === node.id)?.data.semModelV4?.parameters?.[0]).toEqual(loadingEntry);
    expect(edgeAuthoring(reopened.edges.find((candidate) => candidate.id === edge.id))?.parameters?.[0]).toEqual(pathEntry);
  });
});

function edgeAuthoring(edge: { data?: unknown } | undefined) {
  return (edge?.data as PathEdgeData | undefined)?.semModelV4ParameterAuthoring;
}
