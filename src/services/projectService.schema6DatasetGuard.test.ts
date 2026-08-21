import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useInternalProjectArchiveV6Session } from "../internalProjectArchiveV6SessionStore";
import { useWorkspace } from "../store";
import {
  activateNativeDataset,
  applyNativeDatasetTransformation,
  getNativeDatasetRows,
  importNativeDataset,
  importNativeValidationFixture,
  previewNativeDatasetTransformation,
  profileNativeDatasetGroups,
  recodeNativeDatasetColumn,
  updateNativeColumnMetadata,
} from "./projectService";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open, save: vi.fn() }));

const boundSession = {
  kind: "internal_schema6_read_only",
  access: "read_only",
  project: {
    schema_version: 6,
    project_id: "00000000-0000-0000-0000-000000000601",
    origin: { kind: "new_project" },
    sem_generation: "general_sem_v1",
  },
  standardActivation: { modelIds: ["model:general-sem"], sourceArchiveSha256: "a".repeat(64) },
} as never;

describe("legacy dataset service boundary during General SEM authority", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.open.mockReset();
    useWorkspace.setState({ generalSemPublicationPending: false });
    useInternalProjectArchiveV6Session.setState({ session: boundSession });
  });

  afterEach(() => {
    useInternalProjectArchiveV6Session.setState({ session: null });
    useWorkspace.setState({ generalSemPublicationPending: false });
  });

  it("blocks import, metadata, activation, recode, transform, and legacy reads before any native call", async () => {
    const calls = [
      () => importNativeDataset(),
      () => importNativeValidationFixture(),
      () => updateNativeColumnMetadata("dataset", "x", {} as never),
      () => activateNativeDataset("dataset"),
      () => recodeNativeDatasetColumn("dataset", {} as never),
      () => previewNativeDatasetTransformation("dataset", {} as never),
      () => applyNativeDatasetTransformation("dataset", {} as never, "derived"),
      () => getNativeDatasetRows("dataset", 0, 100),
      () => profileNativeDatasetGroups("dataset", "group", ["x"]),
    ];
    for (const call of calls) {
      await expect(call()).rejects.toMatchObject({
        code: "schema6_general_sem.legacy_dataset_operation_blocked",
      });
    }
    expect(mocks.open).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("also blocks mutation services while a fresh General SEM publication is pending", async () => {
    useInternalProjectArchiveV6Session.setState({ session: null });
    useWorkspace.setState({ generalSemPublicationPending: true });
    await expect(importNativeDataset()).rejects.toThrow(/publication is validating and committing/);
    await expect(recodeNativeDatasetColumn("dataset", {} as never)).rejects.toThrow(/publication is validating and committing/);
    expect(mocks.open).not.toHaveBeenCalled();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});
