import { describe, expect, it } from "vitest";
import { completedCtaPlsRun } from "./nativeCtaPls.testFixture";
import {
  buildNativeResultNavigation,
  nativeCtaPlsResultProjection,
  nativeResultTables,
} from "./nativeResults";

describe("native CTA-PLS result contract", () => {
  it("projects the exact bounded payload into accessible summary, tetrad, and scope tables", () => {
    const run = completedCtaPlsRun();
    expect(nativeCtaPlsResultProjection(run)).toMatchObject({
      methodVersion: "cta_pls_tetrad_v1",
      usedObservations: 80,
      omittedObservations: 2,
      blocks: [{ constructId: "x", quadruples: 1, tetrads: 3 }],
      maxAbsoluteTetradByConstruct: { x: 0.01 },
    });
    const tables = nativeResultTables(run);
    expect(tables.filter((table) => table.id.startsWith("cta_pls_")).map((table) => table.id)).toEqual([
      "cta_pls_summary",
      "cta_pls_tetrads",
      "cta_pls_scope",
    ]);
    expect(tables.find((table) => table.id === "cta_pls_summary")?.rows).toEqual([
      ["Composite X", "x1, x2, x3, x4", "1", "3", "0.010000"],
    ]);
    expect(tables.find((table) => table.id === "cta_pls_tetrads")?.rows).toHaveLength(3);
    expect(tables.find((table) => table.id === "cta_pls_scope")?.rows).toEqual(expect.arrayContaining([
      ["Method version", "cta_pls_tetrad_v1"],
      ["Excluded inference", "Bootstrap, permutation, asymptotic, and vanishing-tetrad decisions"],
    ]));

    const navigation = buildNativeResultNavigation(run);
    expect(navigation.defaultItemId).toBe("cta_pls_summary");
    expect(navigation.groups.find((group) => group.id === "assessment")?.items.map((item) => item.id)).toEqual([
      "cta_pls_summary",
      "cta_pls_tetrads",
      "cta_pls_scope",
    ]);
  });

  it.each([
    ["method version", (run: ReturnType<typeof completedCtaPlsRun>) => { run.result!.cta_pls!.method_version = "cta_pls_tetrad_v0"; }],
    ["dataset method", (run: ReturnType<typeof completedCtaPlsRun>) => { run.provenance!.settings.method = "pls_pm"; }],
    ["pairing identity", (run: ReturnType<typeof completedCtaPlsRun>) => { run.result!.cta_pls!.estimates[0].pairing = "unknown"; }],
    ["absolute value", (run: ReturnType<typeof completedCtaPlsRun>) => { run.result!.cta_pls!.estimates[0].absolute_tetrad = 10; }],
    ["three-pair algebra", (run: ReturnType<typeof completedCtaPlsRun>) => { run.result!.cta_pls!.estimates[0].tetrad = 0.02; run.result!.cta_pls!.estimates[0].absolute_tetrad = 0.02; run.result!.cta_pls!.max_absolute_tetrad_by_construct.x = 0.02; }],
    ["block maximum", (run: ReturnType<typeof completedCtaPlsRun>) => { run.result!.cta_pls!.max_absolute_tetrad_by_construct.x = 10; }],
    ["resampling", (run: ReturnType<typeof completedCtaPlsRun>) => { run.provenance!.settings.permutation_samples = 999; }],
  ])("fails closed when %s is altered", (_label, mutate) => {
    const run = completedCtaPlsRun();
    mutate(run);
    expect(nativeCtaPlsResultProjection(run)).toBeNull();
    expect(nativeResultTables(run)).toEqual([]);
  });
});
