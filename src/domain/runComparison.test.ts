import { describe, expect, it } from "vitest";
import { compareRuns } from "./runComparison";
import type { AnalysisRun, PlsResult } from "../types";

const result = (path: number, r2: number): PlsResult => ({
  method_version: "pls_pm_v1",
  converged: true,
  iterations: 3,
  used_observations: 20,
  omitted_observations: 0,
  outer_estimates: [],
  paths: [{ source: "x", target: "y", coefficient: path }],
  effects: [],
  r_squared: { y: r2 },
  warnings: [],
});

const run = (id: string, path: number, r2: number): AnalysisRun => ({
  id,
  name: id,
  method: "PLS-SEM",
  createdAt: "2026-07-19T00:00:00.000Z",
  seed: 1,
  status: "completed",
  warnings: [],
  fingerprint: "fixture",
  result: result(path, r2),
});

describe("run comparison", () => {
  it("compares R2 and path coefficients with deltas", () => {
    expect(compareRuns(run("a", 0.4, 0.16), run("b", 0.5, 0.25))).toEqual([
      { metric: "R2", item: "y", baseline: "0.160000", comparison: "0.250000", delta: "0.090000" },
      { metric: "Path coefficient", item: "x -> y", baseline: "0.400000", comparison: "0.500000", delta: "0.100000" },
    ]);
  });
});
