import { describe, expect, it } from "vitest";
import { NATIVE_GROUP_EMPTY_STATE_DETAIL } from "./RunHistory";

describe("RunHistory group empty-state copy", () => {
  it("offers the standalone MICOM setup without advertising the retired combined workflow", () => {
    expect(NATIVE_GROUP_EMPTY_STATE_DETAIL).toContain("Configure MICOM v3.1");
    expect(NATIVE_GROUP_EMPTY_STATE_DETAIL).not.toContain("MICOM/MGA");
    expect(NATIVE_GROUP_EMPTY_STATE_DETAIL).not.toContain("MICOM + MGA");
  });
});
