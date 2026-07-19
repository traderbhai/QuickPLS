import type { AnalysisRun } from "../types";

export function completedSamplePlsRun(): AnalysisRun {
  return {
    id: "v11-smoke-completed-pls",
    name: "PLS path modeling core run",
    method: "PLS path modeling core",
    createdAt: "2026-07-19T12:00:00.000Z",
    seed: 20260718,
    status: "completed",
    warnings: ["Validated for the documented QuickPLS v1.0.0 supported scope; unsupported shapes remain blocked or explicitly marked."],
    fingerprint: "v11-smoke",
    result: {
      method_version: "pls_pm_v1+v11_smoke_fixture",
      converged: true,
      iterations: 5,
      used_observations: 5,
      omitted_observations: 0,
      outer_estimates: [
        { construct: "competence", indicator: "COMP1", weight: 0.351, loading: 0.842 },
        { construct: "competence", indicator: "COMP2", weight: 0.337, loading: 0.811 },
        { construct: "competence", indicator: "COMP3", weight: 0.329, loading: 0.786 },
        { construct: "likeability", indicator: "LIKE1", weight: 0.511, loading: 0.874 },
        { construct: "likeability", indicator: "LIKE2", weight: 0.497, loading: 0.861 },
        { construct: "satisfaction", indicator: "CUSA1", weight: 0.502, loading: 0.902 },
        { construct: "satisfaction", indicator: "CUSA2", weight: 0.491, loading: 0.888 },
        { construct: "loyalty", indicator: "CUSL1", weight: 0.514, loading: 0.913 },
        { construct: "loyalty", indicator: "CUSL2", weight: 0.486, loading: 0.894 },
      ],
      paths: [
        { source: "competence", target: "satisfaction", coefficient: 0.403 },
        { source: "likeability", target: "satisfaction", coefficient: 0.327 },
        { source: "competence", target: "loyalty", coefficient: 0.116 },
        { source: "likeability", target: "loyalty", coefficient: 0.172 },
        { source: "satisfaction", target: "loyalty", coefficient: 0.544 },
      ],
      effects: [
        { source: "competence", target: "satisfaction", direct: 0.403, indirect: 0, total: 0.403 },
        { source: "likeability", target: "satisfaction", direct: 0.327, indirect: 0, total: 0.327 },
        { source: "competence", target: "loyalty", direct: 0.116, indirect: 0.219, total: 0.335 },
        { source: "likeability", target: "loyalty", direct: 0.172, indirect: 0.178, total: 0.350 },
        { source: "satisfaction", target: "loyalty", direct: 0.544, indirect: 0, total: 0.544 },
      ],
      r_squared: {
        satisfaction: 0.544,
        loyalty: 0.617,
      },
      warnings: [],
    },
  };
}
