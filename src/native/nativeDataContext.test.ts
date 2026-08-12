import { describe, expect, it } from "vitest";
import { nativeDataContextSelection, type NativeDataContextTarget } from "./nativeDataContext";

describe("native Data context targets", () => {
  it.each<[NativeDataContextTarget, string, number]>([
    [{ kind: "variable", column: "A" }, "variable", 1],
    [{ kind: "dataset" }, "dataset", 1],
    [{ kind: "none" }, "none", 0],
  ])("maps %j to the registry selection %s", (target, kind, count) => {
    expect(nativeDataContextSelection(target)).toEqual({ kind, count });
  });

  it("retains the exact variable independently of the registry selection shape", () => {
    const target = { kind: "variable", column: "indicator_B" } as const;

    expect(target.column).toBe("indicator_B");
    expect(nativeDataContextSelection(target)).toEqual({ kind: "variable", count: 1 });
  });
});
