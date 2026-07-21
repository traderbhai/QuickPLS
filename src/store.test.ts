import { beforeEach, describe, expect, it } from "vitest";
import { methods } from "./data/sample";
import { useWorkspace } from "./store";
import type { AnalysisUiSettings, PlsResult } from "./types";

const minimalResult: PlsResult = {
  method_version: "pls_pm_v1",
  converged: true,
  iterations: 1,
  used_observations: 5,
  omitted_observations: 0,
  outer_estimates: [],
  paths: [],
  effects: [],
  r_squared: {},
  warnings: [],
};

describe("model editor state", () => {
  beforeEach(() => useWorkspace.getState().resetProject());

  it("keeps promoted extended PLS methods visible in the desktop catalog", () => {
    const unsupported = new Set(methods.filter((method) => method.status === "unsupported").map((method) => method.id));
    for (const method of []) {
      expect(unsupported.has(method)).toBe(true);
    }
    expect(methods.find((method) => method.id === "wpls")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "cca")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "plsc")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "endogeneity")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "nonlinear_effects")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "moderated_mediation")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "cta_pls")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "predict")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "mga")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "ipma")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "regression")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "nca")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "cbsem")?.status).toBe("validated");
    expect(methods.find((method) => method.id === "gsca")?.status).toBe("validated");
  });

  it("supports undo and redo for construct creation", () => {
    const originalCount = useWorkspace.getState().nodes.length;
    useWorkspace.getState().addConstruct({ x: 20, y: 30 });
    expect(useWorkspace.getState().nodes).toHaveLength(originalCount + 1);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes).toHaveLength(originalCount);
    useWorkspace.getState().redo();
    expect(useWorkspace.getState().nodes).toHaveLength(originalCount + 1);
  });

  it("starts with a pure model diagram and no pre-run result values", () => {
    const state = useWorkspace.getState();
    expect(state.runs).toHaveLength(0);
    expect(state.nodes.every((node) => node.data.score === undefined && node.data.resultR2 === undefined && node.data.resultLoadings === undefined)).toBe(true);
    expect(state.edges.every((edge) => edge.label === "Path")).toBe(true);
  });

  it("prevents self paths and duplicate directed paths", () => {
    const before = useWorkspace.getState().edges.length;
    useWorkspace.getState().onConnect({ source: "competence", target: "competence", sourceHandle: null, targetHandle: null });
    expect(useWorkspace.getState().edges).toHaveLength(before);
    useWorkspace.getState().onConnect({ source: "competence", target: "satisfaction", sourceHandle: null, targetHandle: null });
    expect(useWorkspace.getState().edges).toHaveLength(before);
  });

  it("moves an indicator when it is assigned to another construct", () => {
    useWorkspace.getState().assignIndicator("likeability", "COMP1");
    const state = useWorkspace.getState();
    expect(state.nodes.find((node) => node.id === "competence")?.data.indicators).not.toContain("COMP1");
    expect(state.nodes.find((node) => node.id === "likeability")?.data.indicators).toContain("COMP1");
  });

  it("creates a construct from dropped indicators without duplicate ownership", () => {
    useWorkspace.getState().addConstruct(undefined, ["COMP1", "COMP2"]);
    const state = useWorkspace.getState();
    const created = state.nodes.at(-1)!;
    expect(created.data.indicators).toEqual(["COMP1", "COMP2"]);
    expect(state.nodes.find((node) => node.id === "competence")?.data.indicators).toEqual(["COMP3"]);
    expect(state.nodes.slice(0, -1).every((node) => Math.abs(node.position.x - created.position.x) >= 190 || Math.abs(node.position.y - created.position.y) >= 140)).toBe(true);
  });

  it("nudges dropped constructs to the nearest open space instead of overlapping the model", () => {
    const state = useWorkspace.getState();
    const occupied = state.nodes.find((node) => node.id === "satisfaction")!;
    useWorkspace.getState().addConstruct(occupied.position, ["COMP1"]);
    const created = useWorkspace.getState().nodes.at(-1)!;
    expect(created.position).not.toEqual(occupied.position);
    expect(useWorkspace.getState().nodes.slice(0, -1).every((node) => Math.abs(node.position.x - created.position.x) >= 190 || Math.abs(node.position.y - created.position.y) >= 140)).toBe(true);
  });

  it("creates separate single-item constructs from selected variables in one undo step", () => {
    const originalCount = useWorkspace.getState().nodes.length;
    useWorkspace.getState().addConstructsFromIndicators(["COMP1", "COMP2", "NOT_A_COLUMN", "COMP1"]);
    let state = useWorkspace.getState();
    const created = state.nodes.slice(-2);
    expect(state.nodes).toHaveLength(originalCount + 2);
    expect(created.map((node) => node.data.indicators)).toEqual([["COMP1"], ["COMP2"]]);
    expect(created.map((node) => node.data.label)).toEqual(["COMP1", "COMP2"]);
    expect(created.every((node) => node.selected)).toBe(true);
    expect(state.selectedNodeId).toBe(created[1].id);
    expect(state.nodes.find((node) => node.id === "competence")?.data.indicators).toEqual(["COMP3"]);
    useWorkspace.getState().undo();
    state = useWorkspace.getState();
    expect(state.nodes).toHaveLength(originalCount);
    expect(state.nodes.find((node) => node.id === "competence")?.data.indicators).toEqual(["COMP1", "COMP2", "COMP3"]);
  });

  it("creates grouped constructs by indicator prefix in one undo step", () => {
    const originalCount = useWorkspace.getState().nodes.length;
    useWorkspace.getState().addConstructsFromIndicatorGroups(["COMP1", "COMP2", "COMP3", "LIKE1", "LIKE2", "NOT_A_COLUMN", "COMP1"]);
    let state = useWorkspace.getState();
    const created = state.nodes.slice(-2);
    expect(state.nodes).toHaveLength(originalCount + 2);
    expect(created.map((node) => node.data.shortName)).toEqual(["COMP", "LIKE"]);
    expect(created.map((node) => node.data.indicators)).toEqual([["COMP1", "COMP2", "COMP3"], ["LIKE1", "LIKE2"]]);
    expect(created.every((node) => node.selected)).toBe(true);
    expect(state.selectedNodeId).toBe(created[1].id);
    expect(state.nodes.find((node) => node.id === "competence")?.data.indicators).toEqual([]);
    expect(state.nodes.find((node) => node.id === "likeability")?.data.indicators).toEqual([]);
    useWorkspace.getState().undo();
    state = useWorkspace.getState();
    expect(state.nodes).toHaveLength(originalCount);
    expect(state.nodes.find((node) => node.id === "competence")?.data.indicators).toEqual(["COMP1", "COMP2", "COMP3"]);
    expect(state.nodes.find((node) => node.id === "likeability")?.data.indicators).toEqual(["LIKE1", "LIKE2"]);
  });

  it("draws a path from the explicit path tool action", () => {
    const before = useWorkspace.getState().edges.length;
    useWorkspace.getState().addPath("loyalty", "competence");
    expect(useWorkspace.getState().edges).toHaveLength(before + 1);
    useWorkspace.getState().addPath("loyalty", "competence");
    expect(useWorkspace.getState().edges).toHaveLength(before + 1);
  });

  it("creates a two-stage interaction placeholder without duplicate interaction terms", () => {
    const beforeNodes = useWorkspace.getState().nodes.length;
    useWorkspace.getState().addTwoStageInteraction("competence", "likeability", "satisfaction");
    const state = useWorkspace.getState();
    const interaction = state.nodes.find((node) => node.data.semantic === "interaction")!;
    expect(state.nodes).toHaveLength(beforeNodes + 1);
    expect(interaction.data.interaction).toEqual({ predictor: "competence", moderator: "likeability", outcome: "satisfaction", method: "two_stage_product_score" });
    expect(interaction.data.indicators).toEqual([]);
    expect(state.edges).toContainEqual(expect.objectContaining({ source: interaction.id, target: "satisfaction", label: "Interaction" }));
    useWorkspace.getState().addTwoStageInteraction("competence", "likeability", "satisfaction");
    expect(useWorkspace.getState().nodes.filter((node) => node.data.semantic === "interaction")).toHaveLength(1);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes.filter((node) => node.data.semantic === "interaction")).toHaveLength(0);
  });

  it("keeps a stable edge id when a path endpoint is reconnected", () => {
    const edge = useWorkspace.getState().edges.find((candidate) => candidate.id === "comp-cusa")!;
    useWorkspace.getState().reconnectPath(edge, { source: "satisfaction", target: "competence", sourceHandle: null, targetHandle: null });
    const reconnected = useWorkspace.getState().edges.find((candidate) => candidate.id === edge.id)!;
    expect(reconnected.source).toBe("satisfaction");
    expect(reconnected.target).toBe("competence");
    expect(useWorkspace.getState().selectedEdgeId).toBe(edge.id);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().edges.find((candidate) => candidate.id === edge.id)?.source).toBe("competence");
  });

  it("updates selected path routing and keeps the change undoable", () => {
    useWorkspace.getState().setSelectedEdge("comp-cusa");
    useWorkspace.getState().setSelectedPathRouting("straight");
    expect(useWorkspace.getState().edges.find((edge) => edge.id === "comp-cusa")?.type).toBe("straight");
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().edges.find((edge) => edge.id === "comp-cusa")?.type).toBe("smoothstep");
  });

  it("marks a path as a control with undoable edge metadata", () => {
    useWorkspace.getState().updateEdge("comp-cusa", { label: "Control", data: { role: "control", controlLabel: "Age" } });
    let edge = useWorkspace.getState().edges.find((candidate) => candidate.id === "comp-cusa")!;
    expect(edge.label).toBe("Control");
    expect(edge.data).toEqual({ role: "control", controlLabel: "Age" });
    useWorkspace.getState().undo();
    edge = useWorkspace.getState().edges.find((candidate) => candidate.id === "comp-cusa")!;
    expect(edge.data).toBeUndefined();
  });

  it("marks a construct as higher-order with undoable metadata", () => {
    useWorkspace.getState().updateConstruct("satisfaction", {
      semantic: "higher_order",
      higherOrder: {
        id: "satisfaction",
        components: ["competence", "likeability"],
        method: "repeated_indicators",
        stage_one_recipe: null,
      },
    });
    let construct = useWorkspace.getState().nodes.find((candidate) => candidate.id === "satisfaction")!;
    expect(construct.data.higherOrder).toEqual({
      id: "satisfaction",
      components: ["competence", "likeability"],
      method: "repeated_indicators",
      stage_one_recipe: null,
    });
    useWorkspace.getState().undo();
    construct = useWorkspace.getState().nodes.find((candidate) => candidate.id === "satisfaction")!;
    expect(construct.data.higherOrder).toBeUndefined();
  });

  it("does not reverse a path when the opposite directed path already exists", () => {
    const state = useWorkspace.getState();
    state.loadProject({
      nodes: state.nodes,
      edges: [
        ...state.edges,
        { id: "cusa-comp", source: "satisfaction", target: "competence", label: "Path", type: "smoothstep" },
      ],
      dataset: state.dataset,
    });
    useWorkspace.getState().setSelectedEdge("comp-cusa");
    useWorkspace.getState().reverseSelectedPath();
    const edge = useWorkspace.getState().edges.find((candidate) => candidate.id === "comp-cusa")!;
    expect(edge.source).toBe("competence");
    expect(edge.target).toBe("satisfaction");
  });

  it("rejects non-dataset indicators from external drop payloads", () => {
    useWorkspace.getState().addConstruct(undefined, ["COMP1", "NOT_A_COLUMN", 7 as unknown as string]);
    expect(useWorkspace.getState().nodes.at(-1)?.data.indicators).toEqual(["COMP1"]);
  });

  it("normalizes permutation samples to disabled or the supported range", () => {
    useWorkspace.getState().setAnalysisSettings({ permutationSamples: 1 });
    expect(useWorkspace.getState().analysisSettings.permutationSamples).toBe(99);
    useWorkspace.getState().setAnalysisSettings({ permutationSamples: 20_000 });
    expect(useWorkspace.getState().analysisSettings.permutationSamples).toBe(10_000);
    useWorkspace.getState().setAnalysisSettings({ permutationSamples: 0 });
    expect(useWorkspace.getState().analysisSettings.permutationSamples).toBe(0);
  });

  it("normalizes studentized bootstrap to qualified odd inner counts", () => {
    useWorkspace.getState().setAnalysisSettings({ studentizedInnerSamples: 100 });
    expect(useWorkspace.getState().analysisSettings.studentizedInnerSamples).toBe(101);
    expect(useWorkspace.getState().analysisSettings.bootstrapSamples).toBe(999);
    useWorkspace.getState().setAnalysisSettings({ bootstrapSamples: 0, studentizedInnerSamples: 0 });
    expect(useWorkspace.getState().analysisSettings.studentizedInnerSamples).toBe(0);
    expect(useWorkspace.getState().analysisSettings.bootstrapSamples).toBe(0);
  });

  it("persists supported method settings and normalizes unknown method ids", () => {
    useWorkspace.getState().setAnalysisSettings({ method: "wpls", caseWeightColumn: "COMP1" });
    expect(useWorkspace.getState().analysisSettings.method).toBe("wpls");
    expect(useWorkspace.getState().analysisSettings.caseWeightColumn).toBe("COMP1");
    useWorkspace.getState().loadProject({
      nodes: useWorkspace.getState().nodes,
      edges: useWorkspace.getState().edges,
      dataset: useWorkspace.getState().dataset,
      analysisSettings: { method: "not_a_method", caseWeightColumn: "  " } as unknown as AnalysisUiSettings,
    });
    expect(useWorkspace.getState().analysisSettings.method).toBe("pls_pm");
    expect(useWorkspace.getState().analysisSettings.caseWeightColumn).toBeNull();
  });

  it("keeps diagram estimates explicit and clears them on project load", () => {
    useWorkspace.getState().addRun({
      id: "run-1",
      name: "PLS-SEM run",
      method: "PLS-SEM",
      createdAt: "2026-07-19T00:00:00.000Z",
      seed: 1,
      status: "completed",
      warnings: [],
      fingerprint: "fixture",
      result: minimalResult,
    });
    expect(useWorkspace.getState().selectedResultRunId).toBe("run-1");
    expect(useWorkspace.getState().diagramOverlaySettings.selectedRunId).toBe("run-1");
    expect(useWorkspace.getState().diagramOverlaySettings.mode).toBe("paths_r2");
    useWorkspace.getState().setSelectedResultRun(null);
    expect(useWorkspace.getState().selectedResultRunId).toBeNull();
    expect(useWorkspace.getState().diagramOverlaySettings.selectedRunId).toBeNull();
    useWorkspace.getState().setSelectedResultRun("run-1");
    useWorkspace.getState().loadProject({
      nodes: useWorkspace.getState().nodes,
      edges: useWorkspace.getState().edges,
      dataset: useWorkspace.getState().dataset,
      runs: useWorkspace.getState().runs,
    });
    expect(useWorkspace.getState().runs).toHaveLength(1);
    expect(useWorkspace.getState().selectedResultRunId).toBeNull();
    expect(useWorkspace.getState().diagramOverlaySettings.selectedRunId).toBeNull();
  });

  it("stores covariance display arcs separately from structural path validation", () => {
    const before = useWorkspace.getState().edges.length;
    useWorkspace.getState().addCovariance("competence", "likeability");
    let state = useWorkspace.getState();
    expect(state.edges).toHaveLength(before + 1);
    expect(state.edges.at(-1)?.data).toEqual({ role: "covariance" });
    useWorkspace.getState().addCovariance("likeability", "competence");
    expect(useWorkspace.getState().edges).toHaveLength(before + 1);
    useWorkspace.getState().undo();
    state = useWorkspace.getState();
    expect(state.edges).toHaveLength(before);
  });

  it("loads legacy projects with SEM diagram defaults", () => {
    const current = useWorkspace.getState();
    current.loadProject({ nodes: current.nodes, edges: current.edges, dataset: current.dataset });
    const state = useWorkspace.getState();
    expect(state.diagramMode).toBe("sem");
    expect(state.diagramTool).toBe("select");
    expect(state.publicationDiagramSettings.mode).toBe("smartpls_result");
    expect(state.publicationDiagramSettings.palette).toBe("grayscale");
    expect(state.publicationDiagramSettings.layoutSource).toBe("current_canvas");
    expect(state.diagramLayout.diagramVersion).toBe("sem_designer_v1");
    expect(state.diagramLayout.constructLayouts.competence).toMatchObject({ x: state.nodes.find((node) => node.id === "competence")?.position.x });
  });

  it("persists and resets indicator layout independently from the engine model", () => {
    const originalIndicators = useWorkspace.getState().nodes.find((node) => node.id === "competence")?.data.indicators;
    useWorkspace.getState().checkpoint();
    useWorkspace.getState().moveIndicator("competence", "COMP1", { x: 42, y: 57 });
    let state = useWorkspace.getState();
    expect(state.diagramLayout.indicatorLayouts.competence.COMP1).toMatchObject({ side: "free", x: 42, y: 57, pinned: true });
    expect(state.nodes.find((node) => node.id === "competence")?.data.indicators).toEqual(originalIndicators);
    useWorkspace.getState().undo();
    state = useWorkspace.getState();
    expect(state.diagramLayout.indicatorLayouts.competence.COMP1.side).not.toBe("free");
    useWorkspace.getState().setIndicatorSide("competence", "COMP1", "right");
    expect(useWorkspace.getState().diagramLayout.indicatorLayouts.competence.COMP1).toMatchObject({ side: "right", x: undefined, y: undefined, pinned: true });
    useWorkspace.getState().resetIndicatorLayout("competence", "COMP1");
    expect(useWorkspace.getState().diagramLayout.indicatorLayouts.competence.COMP1.side).not.toBe("free");
  });

  it("places all construct indicators on one side without changing the engine model", () => {
    const originalIndicators = useWorkspace.getState().nodes.find((node) => node.id === "competence")?.data.indicators;
    useWorkspace.getState().moveIndicator("competence", "COMP1", { x: 42, y: 57 });
    useWorkspace.getState().setConstructIndicatorSide("competence", "right");
    let state = useWorkspace.getState();
    expect(Object.values(state.diagramLayout.indicatorLayouts.competence).map((layout) => layout.side)).toEqual(["right", "right", "right"]);
    expect(state.diagramLayout.indicatorLayouts.competence.COMP1).toMatchObject({ x: undefined, y: undefined, pinned: true });
    expect(state.nodes.find((node) => node.id === "competence")?.data.indicators).toEqual(originalIndicators);

    useWorkspace.getState().undo();
    state = useWorkspace.getState();
    expect(state.diagramLayout.indicatorLayouts.competence.COMP1).toMatchObject({ side: "free", x: 42, y: 57, pinned: true });
  });

  it("aligns selected constructs and supports undo", () => {
    const state = useWorkspace.getState();
    state.loadProject({
      nodes: state.nodes.map((node, index) => ({
        ...node,
        selected: index < 2,
        position: index === 0 ? { x: 120, y: 40 } : index === 1 ? { x: 340, y: 160 } : node.position,
      })),
      edges: state.edges,
      dataset: state.dataset,
    });
    useWorkspace.getState().setSelectedNode(null);
    useWorkspace.getState().alignSelectedConstructs("left");
    const selected = useWorkspace.getState().nodes.filter((node) => node.selected);
    expect(selected.map((node) => node.position.x)).toEqual([120, 120]);
    useWorkspace.getState().undo();
    const restored = useWorkspace.getState().nodes.filter((node) => node.selected);
    expect(restored.map((node) => node.position.x)).toEqual([120, 340]);
    expect(restored[0].position.y).not.toBe(restored[1].position.y);
  });

  it("distributes selected constructs evenly by center point", () => {
    const state = useWorkspace.getState();
    state.loadProject({
      nodes: state.nodes.map((node, index) => ({
        ...node,
        selected: index < 3,
        position: index === 0 ? { x: 0, y: 0 } : index === 1 ? { x: 50, y: 100 } : index === 2 ? { x: 300, y: 210 } : node.position,
      })),
      edges: state.edges,
      dataset: state.dataset,
    });
    useWorkspace.getState().setSelectedNode(null);
    useWorkspace.getState().distributeSelectedConstructs("horizontal");
    const selected = useWorkspace.getState().nodes.filter((node) => node.selected).sort((left, right) => left.position.x - right.position.x);
    const centers = selected.map((node) => node.position.x + 85);
    expect(centers).toEqual([85, 235, 385]);
  });

  it("persists edge label offsets and supports reset with undo", () => {
    const edgeId = "competence-satisfaction";
    useWorkspace.getState().checkpoint();
    useWorkspace.getState().setEdgeLabelOffset(edgeId, { x: 12, y: -10 });
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edgeId].labelOffset).toEqual({ x: 12, y: -10 });
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edgeId]?.labelOffset).toBeUndefined();
    useWorkspace.getState().nudgeEdgeLabel(edgeId, { x: 18, y: -16 });
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edgeId].labelOffset).toEqual({ x: 18, y: -16 });
    useWorkspace.getState().nudgeEdgeLabel(edgeId, { x: 0, y: 16 });
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edgeId].labelOffset).toEqual({ x: 18, y: 0 });
    useWorkspace.getState().resetEdgeLabel(edgeId);
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edgeId].labelOffset).toBeUndefined();
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().diagramLayout.edgeLayouts[edgeId].labelOffset).toEqual({ x: 18, y: 0 });
  });
});
