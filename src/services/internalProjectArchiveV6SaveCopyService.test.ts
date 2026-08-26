import { beforeEach, describe, expect, it, vi } from "vitest";
import type { InternalProjectArchiveV6ReadSnapshotV1 } from "../domain/internalProjectArchiveV6Read";
import type { InternalProjectArchiveV6Wire } from "../domain/internalProjectArchiveV6Wire";
import {
  saveInternalProjectArchiveV6Copy,
  saveInternalProjectArchiveV6CopyAt,
} from "./internalProjectArchiveV6SaveCopyService";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), save: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ save: mocks.save }));

const project = {
  schema_version: 6,
  project_id: "00000000-0000-0000-0000-000000000601",
  name: "Save-copy service fixture",
  created_at: "2026-08-15T10:00:00Z",
  modified_at: "2026-08-15T10:01:00Z",
  origin: { kind: "new_project" },
} as unknown as InternalProjectArchiveV6Wire;
const snapshot = {
  archivePath: "D:\\projects\\study-v6.qpls",
  archiveSha256: "a".repeat(64),
  project,
} as unknown as InternalProjectArchiveV6ReadSnapshotV1;
const blocked = {
  status: "blocked",
  diagnostic: {
    code: "schema6_save_copy.destination_exists",
    message: "Destination exists.",
    correctiveAction: "Choose a new filename.",
  },
};

function candidateAuthority(qualified: boolean) {
  return {
    schemaVersion: 1,
    state: qualified ? "release_qualified_candidate" : "labs_only",
    standardSurfaceAuthorized: qualified,
    embeddedDocumentSha256: "a".repeat(64),
    authorityBindingSha256: qualified ? "b".repeat(64) : null,
    candidateCommitSha: qualified ? "c".repeat(40) : null,
    candidateVersion: qualified ? "2.56.0" : null,
    qualificationPlanSha256: qualified ? "d".repeat(64) : null,
    gateBindingSha256: qualified ? "e".repeat(64) : null,
    capabilityIndexSha256: qualified ? "f".repeat(64) : null,
    prepackageManifestSetSha256: qualified ? "1".repeat(64) : null,
    exactProfileCells: qualified
      ? ["mga.general_sem_pls.v1::point_estimation"]
      : [],
  };
}

const labsCandidateAuthority = vi.fn(async () => candidateAuthority(false));
const standardCandidateAuthority = vi.fn(async () => candidateAuthority(true));
const labsDependencies = { candidateAuthority: labsCandidateAuthority };
const standardDependencies = { candidateAuthority: standardCandidateAuthority };

describe("Internal/Labs schema-6 Save copy service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.save.mockReset();
    labsCandidateAuthority.mockClear();
    standardCandidateAuthority.mockClear();
  });

  it("invokes only the dedicated new-copy command with current snapshot identity and detached project", async () => {
    mocks.invoke.mockResolvedValue(blocked);
    const destination = "D:\\projects\\study-v6-model-copy.qpls";

    await expect(saveInternalProjectArchiveV6CopyAt(
      snapshot,
      project,
      destination,
      labsDependencies,
    ))
      .resolves.toEqual(blocked);
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith("save_internal_project_archive_v6_copy", {
      request: expect.objectContaining({
        surface: "internal_labs",
        experimentalLabsEnabled: true,
        sourceArchivePath: snapshot.archivePath,
        expectedSourceArchiveSha256: snapshot.archiveSha256,
        destinationArchivePath: destination,
        project: expect.objectContaining({ schema_version: 6, project_id: project.project_id }),
      }),
    });
    expect(mocks.invoke).not.toHaveBeenCalledWith("save_project", expect.anything());
  });

  it("uses Standard(false) for save/reopen when immutable authority is qualified", async () => {
    mocks.invoke.mockResolvedValue(blocked);
    const destination = "D:\\projects\\study-v6-standard-copy.qpls";

    await expect(
      saveInternalProjectArchiveV6CopyAt(
        snapshot,
        project,
        destination,
        standardDependencies,
      ),
    ).resolves.toEqual(blocked);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "save_internal_project_archive_v6_copy",
      {
        request: expect.objectContaining({
          surface: "standard_multimod_v1",
          experimentalLabsEnabled: false,
          destinationArchivePath: destination,
        }),
      },
    );
  });

  it("does nothing when the new-destination chooser is cancelled", async () => {
    mocks.save.mockResolvedValue(null);

    await expect(
      saveInternalProjectArchiveV6Copy(snapshot, project, labsDependencies),
    ).resolves.toBeNull();
    expect(mocks.save).toHaveBeenCalledWith({
      defaultPath: "D:\\projects\\study-v6-model-copy.qpls",
      filters: [{ name: "QuickPLS schema-6 ZIP project", extensions: ["qpls"] }],
    });
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(labsCandidateAuthority).not.toHaveBeenCalled();
  });

  it("rejects malformed native success responses before session rotation", async () => {
    mocks.invoke.mockResolvedValue({ status: "ok", value: { persistence: "persisted_new_copy" } });

    await expect(saveInternalProjectArchiveV6CopyAt(
      snapshot,
      project,
      "D:\\projects\\new-copy.qpls",
      labsDependencies,
    )).rejects.toMatchObject({ code: "schema6_save_copy.field_missing" });
  });
});
