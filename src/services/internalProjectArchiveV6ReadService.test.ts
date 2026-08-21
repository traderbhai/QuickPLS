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

describe("Internal/Labs schema-6 ZIP read service", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.open.mockReset();
  });

  it("uses only the dedicated strict schema-6 inspection command", async () => {
    mocks.invoke.mockResolvedValue(blocked);

    await expect(inspectInternalProjectArchiveV6At("D:\\projects\\study-v6.qpls"))
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

    await expect(openInternalProjectArchiveV6()).resolves.toBeNull();
    expect(mocks.invoke).not.toHaveBeenCalled();
  });

  it("opens only .qpls files and keeps the result ephemeral", async () => {
    mocks.open.mockResolvedValue("D:\\projects\\study-v6.qpls");
    mocks.invoke.mockResolvedValue(blocked);

    await expect(openInternalProjectArchiveV6()).resolves.toEqual(blocked);
    expect(mocks.open).toHaveBeenCalledWith({
      multiple: false,
      filters: [{ name: "QuickPLS schema-6 ZIP project", extensions: ["qpls"] }],
    });
    expect(mocks.invoke).toHaveBeenCalledTimes(1);
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

    await expect(inspectInternalProjectArchiveV6At("D:\\projects\\study-v6.qpls"))
      .rejects.toMatchObject({ code: "schema6_archive_read.field_missing" });
  });
});
