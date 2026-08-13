import { describe, expect, it } from "vitest";
import { isStandaloneNativeAnalysis } from "./nativeStandaloneAnalysis";

describe("standalone native analysis identity", () => {
  it("recognizes NCA, PCA, and OLS without treating model-bound calculations as standalone", () => {
    expect(isStandaloneNativeAnalysis("nca")).toBe(true);
    expect(isStandaloneNativeAnalysis("pca")).toBe(true);
    expect(isStandaloneNativeAnalysis("regression")).toBe(true);
    expect(isStandaloneNativeAnalysis("pls_pm")).toBe(false);
    expect(isStandaloneNativeAnalysis("ipma")).toBe(false);
    expect(isStandaloneNativeAnalysis(null)).toBe(false);
  });
});
