import { beforeEach, describe, expect, it } from "vitest";
import { useWorkspace } from "./store";

const PROJECT_ID = "60000001-0000-4000-8000-000000000001";

function freshProject(): void {
  const state = useWorkspace.getState();
  state.closeProject();
  state.setProjectMeta("Fresh General SEM study", null, PROJECT_ID);
}

describe("General SEM fresh-project draft authority", () => {
  beforeEach(freshProject);

  it("binds only an empty unsaved project with the exact native identity", () => {
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(true);
    expect(useWorkspace.getState().generalSemProjectDraftMode).toEqual({
      schemaVersion: 1,
      semGeneration: "general_sem_v1",
      sourceProjectId: PROJECT_ID,
    });

    useWorkspace.getState().setProjectMeta("Renamed fresh study", null, PROJECT_ID);
    expect(useWorkspace.getState().generalSemProjectDraftMode?.sourceProjectId).toBe(PROJECT_ID);
  });

  it("rejects identity drift and a populated ordinary project", () => {
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode("60000001-0000-4000-8000-000000000099")).toBe(false);
    useWorkspace.getState().addConstruct({ x: 10, y: 20 });
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(false);
    expect(useWorkspace.getState().generalSemProjectDraftMode).toBeNull();
  });

  it("preserves the marker only for an explicitly identity-bound same-project native refresh", () => {
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(true);
    const before = useWorkspace.getState();
    const marker = before.generalSemProjectDraftMode;
    if (!marker) throw new Error("Expected a fresh General SEM draft marker.");
    before.loadProject({
      nodes: before.nodes,
      edges: before.edges,
      dataset: { ...before.dataset, id: "dataset:imported", columns: ["x1"], rowCount: 10 },
      preserveGeneralSemProjectDraftMode: marker,
    });
    useWorkspace.getState().setProjectMeta("Refreshed draft", null, PROJECT_ID);
    expect(useWorkspace.getState().generalSemProjectDraftMode).toEqual(marker);

    useWorkspace.getState().loadProject({
      nodes: [],
      edges: [],
      dataset: useWorkspace.getState().dataset,
      preserveGeneralSemProjectDraftMode: { ...marker, sourceProjectId: "60000001-0000-4000-8000-000000000099" },
    });
    expect(useWorkspace.getState().generalSemProjectDraftMode).toBeNull();
  });

  it("preserves the staged marker for the same project identity and clears it on open/load, reset, and close boundaries", () => {
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(true);
    const marker = useWorkspace.getState().generalSemProjectDraftMode;
    useWorkspace.getState().setProjectMeta("Saved revision", "D:\\study.qpls", PROJECT_ID);
    expect(useWorkspace.getState().generalSemProjectDraftMode).toEqual(marker);

    freshProject();
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(true);
    const current = useWorkspace.getState();
    current.loadProject({ nodes: [], edges: [], dataset: current.dataset });
    expect(useWorkspace.getState().generalSemProjectDraftMode).toBeNull();
    expect(useWorkspace.getState().projectId).toBeNull();

    freshProject();
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(true);
    useWorkspace.getState().resetProject();
    expect(useWorkspace.getState().generalSemProjectDraftMode).toBeNull();

    freshProject();
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(true);
    useWorkspace.getState().closeProject();
    expect(useWorkspace.getState().generalSemProjectDraftMode).toBeNull();
  });

  it("exposes a controller-visible publication lock and clears it at destructive reset boundaries", () => {
    useWorkspace.getState().setGeneralSemPublicationPending(true);
    expect(useWorkspace.getState().generalSemPublicationPending).toBe(true);
    useWorkspace.getState().closeProject();
    expect(useWorkspace.getState().generalSemPublicationPending).toBe(false);

    useWorkspace.getState().setGeneralSemPublicationPending(true);
    useWorkspace.getState().resetProject();
    expect(useWorkspace.getState().generalSemPublicationPending).toBe(false);
  });

  it("authors explicit interaction_v2 terms only inside a fresh General SEM project", () => {
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(true);
    useWorkspace.setState({
      nodes: [
        { id: "x", type: "construct", position: { x: 0, y: 0 }, data: { label: "Predictor", shortName: "X", mode: "reflective", indicators: ["x1"] } },
        { id: "w", type: "construct", position: { x: 0, y: 120 }, data: { label: "Moderator", shortName: "W", mode: "reflective", indicators: ["w1"] } },
        { id: "z", type: "construct", position: { x: 0, y: 240 }, data: { label: "Second moderator", shortName: "Z", mode: "reflective", indicators: ["z1"] } },
        { id: "y", type: "construct", position: { x: 360, y: 100 }, data: { label: "Outcome", shortName: "Y", mode: "reflective", indicators: ["y1"] } },
      ],
      edges: [
        { id: "focal-x-y", source: "x", target: "y" },
        { id: "focal-w-y", source: "w", target: "y" },
      ],
      past: [],
      future: [],
    });

    const first = useWorkspace.getState().addTwoStageInteraction("x", "w", "y");
    const second = useWorkspace.getState().addTwoStageInteraction("x", "z", "y");
    const differentFocal = useWorkspace.getState().addTwoStageInteraction("w", "z", "y");
    expect([first, second, differentFocal]).toEqual([
      expect.objectContaining({ status: "created" }),
      expect.objectContaining({ status: "created" }),
      expect.objectContaining({ status: "created" }),
    ]);

    const interactions = useWorkspace.getState().nodes
      .filter((node) => node.data.semantic === "interaction")
      .map((node) => node.data.interaction);
    expect(interactions).toEqual(expect.arrayContaining([
      expect.objectContaining({
        kind: "interaction_v2",
        operands: ["x", "w"],
        outcome: "y",
        focalRelationId: "focal-x-y",
        canonicalMethod: "two_stage",
        hierarchyPolicy: "strong",
        productIndicator: null,
      }),
      expect.objectContaining({ kind: "interaction_v2", operands: ["x", "z"], focalRelationId: "focal-x-y" }),
      expect.objectContaining({ kind: "interaction_v2", operands: ["w", "z"], focalRelationId: "focal-w-y" }),
    ]));
    expect(interactions.every((interaction) => interaction?.termId?.startsWith("interaction-term:"))).toBe(true);
  });

  it("keeps active General SEM work globally visible and prevents Labs from disappearing", () => {
    useWorkspace.getState().setUiPreferences({ experimentalLabsEnabled: true });
    useWorkspace.getState().setGeneralSemTransientWorkBlocker("job_active");
    expect(useWorkspace.getState().generalSemTransientWorkBlocker).toBe("job_active");

    useWorkspace.getState().setUiPreferences({ experimentalLabsEnabled: false });
    expect(useWorkspace.getState().uiPreferences.experimentalLabsEnabled).toBe(true);

    useWorkspace.getState().setGeneralSemTransientWorkBlocker("temporary_result_pending");
    expect(useWorkspace.getState().generalSemTransientWorkBlocker).toBe("temporary_result_pending");
    useWorkspace.getState().closeProject();
    expect(useWorkspace.getState().generalSemTransientWorkBlocker).toBeNull();
  });
});
