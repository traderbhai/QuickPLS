import { describe, expect, it } from "vitest";
import { nativeRecodeIssueFieldId } from "./NativeRecodeDialog";

describe("NativeRecodeDialog validation focus", () => {
  it.each([
    ["sourceColumn", "recode-source"],
    ["targetColumn", "recode-target"],
    ["targetType", "recode-type"],
    ["targetScale", "recode-scale"],
    ["unmapped", "recode-unmapped"],
    ["mappings", "recode-source-0"],
    ["mappings.2.source", "recode-source-2"],
    ["mappings.3.target", "recode-target-3"],
  ])("links %s to %s", (path, expected) => {
    expect(nativeRecodeIssueFieldId("recode", path)).toBe(expected);
  });

  it("does not invent a focus target for a dataset-level issue", () => {
    expect(nativeRecodeIssueFieldId("recode", "dataset")).toBeNull();
  });
});
