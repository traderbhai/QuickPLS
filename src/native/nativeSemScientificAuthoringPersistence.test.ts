import { beforeEach, describe, expect, it } from "vitest";
import {
  adaptAuthoredNativeWorkbenchToSemModelV4,
  nativeWorkbenchObservedVariableIdV4,
} from "../domain/nativeWorkbenchSemModelV4Adapter";
import {
  confirmNativeSemConstructAuthoringV4,
  confirmNativeSemCovarianceAuthoringV4,
} from "../domain/semModelV4ScientificAuthoring";
import { useWorkspace } from "../store";
import type { NativeCanonicalModelSpec } from "../types";
import { nativeModelSnapshotFromCanonical } from "./nativeCanonicalProject";

describe("SemModelV4 scientific authoring persistence", () => {
  beforeEach(() => useWorkspace.getState().resetProject());

  it("clears and restores an explicit construct decision through the same history ledger", () => {
    const node = useWorkspace.getState().nodes.find((candidate) => candidate.data.indicators.length > 0)!;
    useWorkspace.getState().setConstructEstimandV4(node.id, {
      kind: "common_factor",
      marker_indicator: node.data.indicators[0],
    });
    useWorkspace.getState().setConstructEstimandV4(node.id, { kind: "legacy_estimand_unspecified" });
    expect(useWorkspace.getState().nodes.find((candidate) => candidate.id === node.id)?.data.semModelV4?.construct).toEqual({
      kind: "legacy_estimand_unspecified",
    });
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes.find((candidate) => candidate.id === node.id)?.data.semModelV4?.construct).toMatchObject({
      kind: "common_factor",
      marker_indicator: node.data.indicators[0],
    });
    useWorkspace.getState().redo();
    expect(useWorkspace.getState().nodes.find((candidate) => candidate.id === node.id)?.data.semModelV4?.construct).toEqual({
      kind: "legacy_estimand_unspecified",
    });
  });

  it("keeps one exact residual-classification change deterministic across undo, redo, and reopen", () => {
    const store = useWorkspace.getState();
    store.addCovariance("competence", "likeability");
    let state = useWorkspace.getState();
    const covariance = state.edges.find((edge) => edge.data?.role === "covariance")!;
    const source = state.nodes.find((node) => node.id === covariance.source)!;
    const target = state.nodes.find((node) => node.id === covariance.target)!;
    const authored = confirmNativeSemCovarianceAuthoringV4(covariance, state.nodes, state.edges, {
      kind: "residual_covariance",
      source_indicator: source.data.indicators[0],
      target_indicator: target.data.indicators[0],
    });
    expect(authored.ok).toBe(true);
    if (!authored.ok) return;

    state.updateEdge(covariance.id, { data: authored.edge.data });
    const residualData = useWorkspace.getState().edges.find((edge) => edge.id === covariance.id)!.data;
    expect(residualData?.semModelV4).toMatchObject({
      covariance: {
        left: { kind: "residual_of", id: nativeWorkbenchObservedVariableIdV4(source.data.indicators[0]) },
        right: { kind: "residual_of", id: nativeWorkbenchObservedVariableIdV4(target.data.indicators[0]) },
      },
    });

    useWorkspace.getState().undo();
    expect(useWorkspace.getState().edges.find((edge) => edge.id === covariance.id)?.data?.semModelV4).toMatchObject({
      covariance: { left: null, right: null },
    });
    useWorkspace.getState().redo();
    expect(useWorkspace.getState().edges.find((edge) => edge.id === covariance.id)?.data).toEqual(residualData);

    const beforeReopen = useWorkspace.getState();
    beforeReopen.loadProject({
      nodes: JSON.parse(JSON.stringify(beforeReopen.nodes)),
      edges: JSON.parse(JSON.stringify(beforeReopen.edges)),
      dataset: beforeReopen.dataset,
    });
    expect(useWorkspace.getState().edges.find((edge) => edge.id === covariance.id)?.data).toEqual(residualData);
  });

  it("preserves explicit factor and residual endpoint metadata through canonical presentation reopen", () => {
    const canonical: NativeCanonicalModelSpec = {
      id: "model-a",
      name: "Persistence model",
      constructs: [
        { id: "x", name: "X", short_name: "X", mode: "reflective", indicators: ["x1", "x2"] },
        { id: "y", name: "Y", short_name: "Y", mode: "reflective", indicators: ["y1", "y2"] },
      ],
      paths: [{ source: "x", target: "y" }],
      controls: [],
      interactions: [],
      higher_order_constructs: [],
    };
    const xBase = { id: "x", type: "construct", position: { x: 10, y: 20 }, data: { label: "X", shortName: "X", mode: "reflective" as const, indicators: ["x1", "x2"] } };
    const xResult = confirmNativeSemConstructAuthoringV4(xBase, "common_factor", "x2");
    expect(xResult.ok).toBe(true);
    if (!xResult.ok) return;
    const yBase = { id: "y", type: "construct", position: { x: 200, y: 20 }, data: { label: "Y", shortName: "Y", mode: "reflective" as const, indicators: ["y1", "y2"] } };
    const yResult = confirmNativeSemConstructAuthoringV4(yBase, "common_factor", "y1");
    expect(yResult.ok).toBe(true);
    if (!yResult.ok) return;
    const covarianceBase = { id: "cov-x-y", source: "x", target: "y", type: "default", label: "Residual covariance", data: { role: "covariance" as const } };
    const covarianceResult = confirmNativeSemCovarianceAuthoringV4(covarianceBase, [xResult.node, yResult.node], [covarianceBase], {
      kind: "residual_covariance",
      source_indicator: "x1",
      target_indicator: "y2",
    });
    expect(covarianceResult.ok).toBe(true);
    if (!covarianceResult.ok) return;

    const reopened = nativeModelSnapshotFromCanonical(canonical, {
      nodes: [xResult.node, yResult.node],
      edges: [covarianceResult.edge],
    });
    expect(reopened.nodes.find((node) => node.id === "x")?.data.semModelV4).toEqual(xResult.node.data.semModelV4);
    expect(reopened.edges.find((edge) => edge.id === "cov-x-y")?.data).toEqual(covarianceResult.edge.data);

    const adapted = adaptAuthoredNativeWorkbenchToSemModelV4({
      model_id: canonical.id,
      model_name: canonical.name,
      nodes: [...reopened.nodes].reverse(),
      edges: [...reopened.edges].reverse(),
      data_binding: {
        kind: "raw",
        dataset_id: "data-a",
        missing_data: "listwise_deletion",
        weight: null,
        cluster_variable: null,
        strata_variable: null,
      },
    });
    expect(adapted.ok).toBe(true);
    if (!adapted.ok) return;
    const covariance = adapted.model.relations.find((relation) => relation.kind === "covariance");
    expect(covariance).toMatchObject({
      left: { kind: "residual_of", id: "observed:x1" },
      right: { kind: "residual_of", id: "observed:y2" },
    });
    expect(adapted.trace.edge_objects["cov-x-y"]).toMatchObject({ kind: "scientific_relation" });
  });
});
