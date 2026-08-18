import { renderToStaticMarkup } from "react-dom/server";
import { beforeEach, describe, expect, it } from "vitest";
import { useWorkspace } from "../store";
import type { AnalysisRun, PlsResult } from "../types";
import { FimixPanel, hasGroupPayload, PosPanel } from "./GroupsWorkspace";

const baseResult: PlsResult = {
  method_version: "pls_pm_v1",
  converged: true,
  iterations: 1,
  used_observations: 20,
  omitted_observations: 0,
  outer_estimates: [],
  paths: [],
  effects: [],
  r_squared: {},
  warnings: [],
};

function completedRun(result: PlsResult): AnalysisRun {
  return {
    id: "segmentation-run",
    name: "Segmentation preview",
    method: "Prediction",
    createdAt: "2026-08-13T00:00:00.000Z",
    seed: 7,
    status: "completed",
    warnings: [],
    fingerprint: "fixture",
    result,
  };
}

describe("GroupsWorkspace segmentation claim boundaries", () => {
  beforeEach(() => useWorkspace.getState().resetProject());

  it("labels FIMIX-style output as an Experimental non-EM diagnostic", () => {
    const run = completedRun({
      ...baseResult,
      fimix: {
        method_version: "fimix_pls_v1",
        classes: 2,
        starts: 10,
        iterations: 4,
        log_likelihood: -12,
        aic: 30,
        bic: 33,
        caic: 35,
        entropy: 0.6,
        classes_summary: [],
        memberships: [],
        warnings: [],
      },
    });
    useWorkspace.getState().loadProject({
      nodes: useWorkspace.getState().nodes,
      edges: useWorkspace.getState().edges,
      dataset: useWorkspace.getState().dataset,
      runs: [run],
    });

    expect(hasGroupPayload(run.result)).toBe(true);
    const html = renderToStaticMarkup(<FimixPanel result={run.result!} />);
    expect(html).toContain("FIMIX-style diagnostic");
    expect(html).toContain("Experimental");
    expect(html).toContain("not posterior probabilities or full finite-mixture EM/FIMIX-PLS equivalence");
    expect(html).not.toContain("FIMIX-PLS</strong><span class=\"status-text validated\"");
  });

  it("labels PLS-POS-style output as an Experimental diagnostic", () => {
    const run = completedRun({
      ...baseResult,
      segmentation: {
        method_version: "pls_pos_v1",
        algorithm: "deterministic_score_space",
        requested_segments: 2,
        selected_segments: 2,
        assignment: "fixed deterministic partition",
        observations: 20,
        objective: 1,
        pooled_objective: 2,
        objective_improvement: 0.5,
        min_segment_share: 0.5,
        segment_size_imbalance: 0,
        max_path_separation: 0.2,
        warnings: [],
        segments: [],
        memberships: [],
      },
    });
    useWorkspace.getState().loadProject({
      nodes: useWorkspace.getState().nodes,
      edges: useWorkspace.getState().edges,
      dataset: useWorkspace.getState().dataset,
      runs: [run],
    });

    expect(hasGroupPayload(run.result)).toBe(true);
    const html = renderToStaticMarkup(<PosPanel result={run.result!} />);
    expect(html).toContain("PLS-POS-style diagnostic");
    expect(html).toContain("Experimental");
    expect(html).toContain("does not implement unrestricted published PLS-POS");
    expect(html).not.toContain("PLS-POS</strong><span class=\"status-text validated\"");
  });
});
