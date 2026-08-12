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

  it("caps transient desktop notifications so they cannot obscure the workbench", () => {
    useWorkspace.getState().pushToast({ tone: "info", title: "First" });
    useWorkspace.getState().pushToast({ tone: "success", title: "Second" });
    useWorkspace.getState().pushToast({ tone: "warning", title: "Third" });

    expect(useWorkspace.getState().toasts.map((toast) => toast.title)).toEqual(["Third", "Second"]);
  });

  it("keeps SEM explorer UI preferences separate from numerical history", () => {
    const beforeHistory = useWorkspace.getState().past.length;
    useWorkspace.getState().setExplorerTab("variables");
    useWorkspace.getState().setExplorerWidth(900);
    useWorkspace.getState().setExplorerCollapsed(true);
    const state = useWorkspace.getState();
    expect(state.explorerTab).toBe("variables");
    expect(state.explorerWidth).toBe(430);
    expect(state.explorerCollapsed).toBe(true);
    expect(state.past).toHaveLength(beforeHistory);
  });

  it("switches models atomically while retaining each model graph and presentation", () => {
    const initial = useWorkspace.getState();
    const firstModel = initial.projectModels[0];
    const secondModel = { ...firstModel, id: "alternate-model", name: "Alternate model" };
    const secondNodes = initial.nodes.map((node) => ({
      ...node,
      position: { x: node.position.x + 700, y: node.position.y + 250 },
    }));
    initial.loadProject({
      nodes: initial.nodes,
      edges: initial.edges,
      dataset: initial.dataset,
      projectModels: [firstModel, secondModel],
      activeModelId: firstModel.id,
      modelPresentations: {
        [firstModel.id]: { nodes: initial.nodes, edges: initial.edges, diagramLayout: initial.diagramLayout },
        [secondModel.id]: { nodes: secondNodes, edges: initial.edges, diagramLayout: initial.diagramLayout },
      },
    });
    useWorkspace.getState().updateConstruct("competence", { label: "Edited competence" });
    useWorkspace.getState().onNodesChange([{ id: "competence", type: "position", position: { x: 123, y: 456 } }]);

    expect(useWorkspace.getState().switchProjectModel(secondModel.id)).toBe(true);
    let switched = useWorkspace.getState();
    expect(switched.activeModelId).toBe(secondModel.id);
    expect(switched.nodes.find((node) => node.id === "competence")?.position.x).toBeGreaterThan(600);
    expect(switched.projectModels.find((model) => model.id === firstModel.id)?.constructs.find((construct) => construct.id === "competence")?.name).toBe("Edited competence");

    expect(switched.switchProjectModel(firstModel.id)).toBe(true);
    switched = useWorkspace.getState();
    expect(switched.nodes.find((node) => node.id === "competence")).toMatchObject({
      position: { x: 123, y: 456 },
      data: { label: "Edited competence" },
    });
    expect(switched.past).toEqual([]);
    expect(switched.future).toEqual([]);
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

  it("does not assign manifest indicators to generated higher-order blocks", () => {
    useWorkspace.getState().updateConstruct("satisfaction", {
      semantic: "higher_order",
      higherOrder: {
        id: "satisfaction",
        components: ["competence", "likeability"],
        method: "two_stage",
        stage_one_recipe: null,
      },
    });
    const before = useWorkspace.getState().nodes.map((node) => ({ id: node.id, indicators: [...node.data.indicators] }));

    useWorkspace.getState().assignIndicator("satisfaction", "COMP1");
    useWorkspace.getState().assignIndicators("satisfaction", ["COMP1", "COMP2"]);

    expect(useWorkspace.getState().nodes.map((node) => ({ id: node.id, indicators: node.data.indicators }))).toEqual(before);
  });

  it("keeps the configured grouping variable out of every indicator assignment path", () => {
    useWorkspace.getState().unassignIndicator("competence", "COMP1");
    useWorkspace.getState().setAnalysisSettings({ groupColumn: "COMP1" });

    useWorkspace.getState().assignIndicator("likeability", "COMP1");
    useWorkspace.getState().assignIndicators("likeability", ["COMP1", "COMP2"]);
    useWorkspace.getState().addConstruct(undefined, ["COMP1", "COMP3"]);
    useWorkspace.getState().addConstructsFromIndicators(["COMP1"]);
    useWorkspace.getState().addConstructsFromIndicatorGroups(["COMP1"]);

    const state = useWorkspace.getState();
    expect(state.nodes.flatMap((node) => node.data.indicators)).not.toContain("COMP1");
    expect(state.nodes.find((node) => node.id === "likeability")?.data.indicators).toContain("COMP2");
    expect(state.nodes.at(-1)?.data.indicators).toEqual(["COMP3"]);
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

  it("creates one two-stage moderating effect and atomically adds the moderator main effect", () => {
    useWorkspace.setState((state) => ({ edges: state.edges.filter((edge) => edge.id !== "like-cusa") }));
    const beforeNodes = useWorkspace.getState().nodes.length;
    expect(useWorkspace.getState().edges.some((edge) => edge.source === "likeability" && edge.target === "satisfaction")).toBe(false);
    const created = useWorkspace.getState().addTwoStageInteraction("competence", "likeability", "satisfaction");
    expect(created).toEqual(expect.objectContaining({ status: "created" }));
    const state = useWorkspace.getState();
    const interaction = state.nodes.find((node) => node.data.semantic === "interaction")!;
    expect(state.nodes).toHaveLength(beforeNodes + 1);
    expect(interaction.data.interaction).toEqual({ predictor: "competence", moderator: "likeability", outcome: "satisfaction", method: "two_stage_product_score" });
    expect(interaction.data.indicators).toEqual([]);
    expect(state.edges).toContainEqual(expect.objectContaining({ source: "likeability", target: "satisfaction", label: "Path" }));
    expect(state.edges).toContainEqual(expect.objectContaining({ source: interaction.id, target: "satisfaction", label: "Interaction" }));
    const duplicate = useWorkspace.getState().addTwoStageInteraction("competence", "likeability", "satisfaction");
    expect(duplicate).toEqual({ status: "blocked", reason: "interaction_exists" });
    expect(useWorkspace.getState().nodes.filter((node) => node.data.semantic === "interaction")).toHaveLength(1);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes.filter((node) => node.data.semantic === "interaction")).toHaveLength(0);
    expect(useWorkspace.getState().edges.some((edge) => edge.source === "likeability" && edge.target === "satisfaction")).toBe(false);
  });

  it("requires a focal structural path and does not assign manifest indicators to generated interactions", () => {
    const before = useWorkspace.getState();
    expect(before.addTwoStageInteraction("loyalty", "likeability", "satisfaction")).toEqual({
      status: "blocked",
      reason: "focal_path_missing",
    });
    expect(useWorkspace.getState().nodes.some((node) => node.data.semantic === "interaction")).toBe(false);

    before.addTwoStageInteraction("competence", "likeability", "satisfaction");
    const interaction = useWorkspace.getState().nodes.find((node) => node.data.semantic === "interaction")!;
    useWorkspace.getState().assignIndicator(interaction.id, "COMP1");
    useWorkspace.getState().assignIndicators(interaction.id, ["COMP2", "COMP3"]);
    expect(useWorkspace.getState().nodes.find((node) => node.id === interaction.id)?.data.indicators).toEqual([]);
    const beforeEdges = useWorkspace.getState().edges.length;
    useWorkspace.getState().addPath(interaction.id, "loyalty");
    useWorkspace.getState().addCovariance(interaction.id, "loyalty");
    expect(useWorkspace.getState().edges).toHaveLength(beforeEdges);
  });

  it("returns a truthful blocker instead of creating moderation around control paths", () => {
    useWorkspace.setState((state) => ({
      edges: state.edges.map((edge) => edge.id === "like-cusa"
        ? { ...edge, data: { ...(edge.data ?? {}), role: "control" } }
        : edge),
    }));
    expect(useWorkspace.getState().addTwoStageInteraction("competence", "likeability", "satisfaction")).toEqual({
      status: "blocked",
      reason: "control_paths_unsupported",
    });
    expect(useWorkspace.getState().nodes.some((node) => node.data.semantic === "interaction")).toBe(false);
  });

  it("protects required moderation relationships and cascades interaction removal", () => {
    expect(useWorkspace.getState().addTwoStageInteraction("competence", "likeability", "satisfaction")).toEqual(
      expect.objectContaining({ status: "created" }),
    );
    const interaction = useWorkspace.getState().nodes.find((node) => node.data.semantic === "interaction")!;
    const focal = useWorkspace.getState().edges.find((edge) => edge.source === "competence" && edge.target === "satisfaction")!;
    useWorkspace.getState().setSelectedEdge(focal.id);
    useWorkspace.getState().reverseSelectedPath();
    expect(useWorkspace.getState().edges.find((edge) => edge.id === focal.id)).toMatchObject({ source: "competence", target: "satisfaction" });

    useWorkspace.getState().removeSelection();
    expect(useWorkspace.getState().edges.some((edge) => edge.id === focal.id)).toBe(false);
    expect(useWorkspace.getState().nodes.some((node) => node.id === interaction.id)).toBe(false);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().edges.some((edge) => edge.id === focal.id)).toBe(true);
    expect(useWorkspace.getState().nodes.some((node) => node.id === interaction.id)).toBe(true);

    useWorkspace.getState().setSelectedNode("likeability");
    useWorkspace.getState().removeSelection();
    expect(useWorkspace.getState().nodes.some((node) => node.id === "likeability")).toBe(false);
    expect(useWorkspace.getState().nodes.some((node) => node.data.semantic === "interaction")).toBe(false);
  });

  it("keeps generated interaction semantics and required endpoints immutable in the store", () => {
    useWorkspace.getState().addTwoStageInteraction("competence", "likeability", "satisfaction");
    const interaction = useWorkspace.getState().nodes.find((node) => node.data.semantic === "interaction")!;
    useWorkspace.getState().updateConstruct(interaction.id, { mode: "reflective", indicators: ["COMP1"], semantic: undefined, interaction: undefined });
    expect(useWorkspace.getState().nodes.find((node) => node.id === interaction.id)?.data).toMatchObject({
      mode: "formative",
      indicators: [],
      semantic: "interaction",
      interaction: interaction.data.interaction,
    });

    const productPath = useWorkspace.getState().edges.find((edge) => edge.source === interaction.id && edge.target === "satisfaction")!;
    useWorkspace.getState().updateEdge(productPath.id, { source: "loyalty", target: "competence", data: { role: "covariance" } });
    expect(useWorkspace.getState().edges.find((edge) => edge.id === productPath.id)).toMatchObject({
      source: interaction.id,
      target: "satisfaction",
    });
    expect(useWorkspace.getState().edges.find((edge) => edge.id === productPath.id)?.data?.role).toBeUndefined();
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
    expect(useWorkspace.getState().diagramLayout.edgeLayouts["comp-cusa"]?.pinned).toBe(false);
    useWorkspace.getState().setSelectedPathRouting("smoothstep");
    expect(useWorkspace.getState().diagramLayout.edgeLayouts["comp-cusa"]).toMatchObject({ routing: "orthogonal", pinned: true });
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().edges.find((edge) => edge.id === "comp-cusa")?.type).toBe("straight");
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

  it("creates one bounded two-stage HOC atomically, locks generated semantics, and cascades component deletion", () => {
    const state = useWorkspace.getState();
    state.loadProject({
      dataset: state.dataset,
      nodes: [
        { id: "x", position: { x: 0, y: 0 }, data: { label: "Capability", shortName: "CAP", mode: "reflective", indicators: ["COMP1"] } },
        { id: "z", position: { x: 0, y: 180 }, data: { label: "Likeability", shortName: "LIKE", mode: "reflective", indicators: ["LIKE1"] } },
        { id: "y", position: { x: 500, y: 90 }, data: { label: "Loyalty", shortName: "LOY", mode: "reflective", indicators: ["CUSL1"] } },
      ],
      edges: [],
    });

    const created = useWorkspace.getState().addHigherOrderConstruct({
      name: "Corporate reputation",
      shortName: "REPU",
      components: ["x", "z"],
    });
    expect(created.status).toBe("created");
    if (created.status !== "created") return;
    let hoc = useWorkspace.getState().nodes.find((node) => node.id === created.constructId)!;
    expect(hoc.data).toMatchObject({
      mode: "reflective",
      indicators: [],
      semantic: "higher_order",
      higherOrder: { id: created.constructId, components: ["x", "z"], method: "two_stage", stage_one_recipe: null },
    });

    useWorkspace.getState().updateConstruct(created.constructId, { mode: "formative", indicators: ["CUSL1"], semantic: undefined, higherOrder: undefined });
    hoc = useWorkspace.getState().nodes.find((node) => node.id === created.constructId)!;
    expect(hoc.data).toMatchObject({ mode: "reflective", indicators: [], semantic: "higher_order" });
    expect(hoc.data.higherOrder?.components).toEqual(["x", "z"]);

    useWorkspace.getState().setSelectedNode("x");
    useWorkspace.getState().removeSelection();
    expect(useWorkspace.getState().nodes.map((node) => node.id)).toEqual(["z", "y"]);
    useWorkspace.getState().undo();
    expect(useWorkspace.getState().nodes.map((node) => node.id)).toEqual(["x", "z", "y", created.constructId]);
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

  it("normalizes the bounded joint MICOM and permutation-MGA settings", () => {
    useWorkspace.getState().setAnalysisSettings({
      groupMethods: "micom, mga_permutation, micom",
      groupPermutationSamples: 1_000,
      micomConfiguralConfirmed: true,
    });
    expect(useWorkspace.getState().analysisSettings.groupMethods).toBe("micom,mga_permutation");
    expect(useWorkspace.getState().analysisSettings.groupPermutationSamples).toBe(5_000);
    expect(useWorkspace.getState().analysisSettings.micomConfiguralConfirmed).toBe(true);

    useWorkspace.getState().setAnalysisSettings({ groupMethods: "micom" });
    expect(useWorkspace.getState().analysisSettings.groupMethods).toBe("micom");
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

  it("persists toolbar view preferences without changing the engine model", () => {
    const originalNodes = useWorkspace.getState().nodes;
    const originalEdges = useWorkspace.getState().edges;
    useWorkspace.getState().setDiagramTheme("academic_grayscale");
    useWorkspace.getState().setDiagramGridVisible(false);
    useWorkspace.getState().setDiagramLayoutLocked(true);
    useWorkspace.getState().toggleConstructPinned("competence");
    const state = useWorkspace.getState();
    expect(state.diagramLayout.diagramTheme).toBe("academic_grayscale");
    expect(state.diagramLayout.showGrid).toBe(false);
    expect(state.diagramLayout.layoutLocked).toBe(true);
    expect(state.diagramLayout.constructLayouts.competence.pinned).toBe(true);
    expect(state.nodes).toEqual(originalNodes);
    expect(state.edges).toEqual(originalEdges);
    useWorkspace.getState().setDiagramLayoutLocked(false);
    expect(useWorkspace.getState().diagramLayout.constructLayouts.competence.pinned).toBe(true);
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
