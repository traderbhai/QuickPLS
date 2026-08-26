import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  inspectInternalProjectArchiveV6At,
  openInternalProjectArchiveV6,
  selectQuickPlsProjectArchivePath,
} from "./internalProjectArchiveV6ReadService";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), open: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: mocks.open }));

const blocked = {
  status: "blocked",
  diagnostic: {
    code: "schema6_archive_read.invalid_archive",
    message: "Archive validation failed.",
    correctiveAction: "Restore a trusted schema-6 ZIP.",
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

describe("Internal/Labs schema-6 ZIP read service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.open.mockReset();
    labsCandidateAuthority.mockClear();
    standardCandidateAuthority.mockClear();
  });

  it("uses only the dedicated strict schema-6 inspection command", async () => {
    mocks.invoke.mockResolvedValue(blocked);

    await expect(inspectInternalProjectArchiveV6At(
      "D:\\projects\\study-v6.qpls",
      labsDependencies,
    ))
      .resolves.toEqual(blocked);
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "inspect_internal_project_archive_v6_zip",
      {
        request: {
          surface: "internal_labs",
          experimentalLabsEnabled: true,
          archivePath: "D:\\projects\\study-v6.qpls",
        },
      },
    );
    expect(mocks.invoke).not.toHaveBeenCalledWith("open_project", expect.anything());
  });

  it("returns null on dialog cancellation without invoking a project command", async () => {
    mocks.open.mockResolvedValue(null);

    await expect(openInternalProjectArchiveV6(labsDependencies)).resolves.toBeNull();
    expect(mocks.invoke).not.toHaveBeenCalled();
    expect(labsCandidateAuthority).not.toHaveBeenCalled();
  });

  it("opens only .qpls files and keeps the result ephemeral", async () => {
    mocks.open.mockResolvedValue("D:\\projects\\study-v6.qpls");
    mocks.invoke.mockResolvedValue(blocked);

    await expect(openInternalProjectArchiveV6(labsDependencies)).resolves.toEqual(blocked);
    expect(mocks.open).toHaveBeenCalledWith({
      multiple: false,
      filters: [{ name: "QuickPLS schema-6 ZIP project", extensions: ["qpls"] }],
    });
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
  });

  it("uses Standard(false) in a qualified build without consulting a Labs preference", async () => {
    mocks.invoke.mockResolvedValue(blocked);

    await expect(
      inspectInternalProjectArchiveV6At(
        "D:\\projects\\study-v6.qpls",
        standardDependencies,
      ),
    ).resolves.toEqual(blocked);
    expect(mocks.invoke).toHaveBeenCalledWith(
      "inspect_internal_project_archive_v6_zip",
      {
        request: {
          surface: "standard_multimod_v1",
          experimentalLabsEnabled: false,
          archivePath: "D:\\projects\\study-v6.qpls",
        },
      },
    );
  });

  it("selects one project path without choosing a loader prematurely", async () => {
    mocks.open.mockResolvedValue("D:\\projects\\study.qpls");

    await expect(selectQuickPlsProjectArchivePath()).resolves.toBe("D:\\projects\\study.qpls");
    expect(mocks.open).toHaveBeenCalledWith({
      multiple: false,
      filters: [{ name: "QuickPLS project", extensions: ["qpls"] }],
    });
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("rejects malformed native responses at the service boundary", async () => {
    mocks.invoke.mockResolvedValue({ status: "ok", value: { access: "read_only" } });

    await expect(inspectInternalProjectArchiveV6At(
      "D:\\projects\\study-v6.qpls",
      labsDependencies,
    ))
      .rejects.toMatchObject({ code: "schema6_archive_read.field_missing" });
  });
});
