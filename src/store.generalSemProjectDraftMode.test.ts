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

  it("clears the transient marker on save identity, open/load, reset, and close boundaries", () => {
    expect(useWorkspace.getState().beginGeneralSemProjectDraftMode(PROJECT_ID)).toBe(true);
    useWorkspace.getState().setProjectMeta("Saved incorrectly", "D:\\study.qpls", PROJECT_ID);
    expect(useWorkspace.getState().generalSemProjectDraftMode).toBeNull();

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
