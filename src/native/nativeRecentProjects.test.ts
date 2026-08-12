import { describe, expect, it } from "vitest";
import {
  NATIVE_RECENT_PROJECTS_KEY,
  loadNativeRecentProjects,
  rememberNativeRecentProject,
  storeNativeRecentProjects,
} from "./nativeRecentProjects";

describe("native recent projects", () => {
  it("keeps the newest unique saved path first", () => {
    const previous = [
      { name: "Study", path: "D:\\studies\\study.qpls", openedAt: "2026-08-01T00:00:00.000Z" },
      { name: "Other", path: "D:\\studies\\other.qpls", openedAt: "2026-08-02T00:00:00.000Z" },
    ];
    expect(rememberNativeRecentProject(previous, {
      name: "Study updated",
      path: "d:\\studies\\STUDY.qpls",
      openedAt: "2026-08-09T00:00:00.000Z",
    })).toEqual([
      { name: "Study updated", path: "d:\\studies\\STUDY.qpls", openedAt: "2026-08-09T00:00:00.000Z" },
      previous[1],
    ]);
  });

  it("ignores corrupt or incomplete stored entries", () => {
    expect(loadNativeRecentProjects({ getItem: () => "not-json" })).toEqual([]);
    expect(loadNativeRecentProjects({ getItem: () => JSON.stringify([{ name: "Missing path" }, { name: "Valid", path: "D:\\valid.qpls", openedAt: "now" }]) })).toEqual([
      { name: "Valid", path: "D:\\valid.qpls", openedAt: "now" },
    ]);
  });

  it("writes the versioned launcher list", () => {
    const writes: Array<[string, string]> = [];
    const entries = [{ name: "Study", path: "D:\\study.qpls", openedAt: "now" }];
    storeNativeRecentProjects({ setItem: (key, value) => { writes.push([key, value]); } }, entries);
    expect(writes).toEqual([[NATIVE_RECENT_PROJECTS_KEY, JSON.stringify(entries)]]);
  });
});
