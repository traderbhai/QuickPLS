import type { AnalysisRun } from "../types";
import {
  NATIVE_CTA_PLS_COVARIANCE_VERSION,
  NATIVE_CTA_PLS_ESTIMATION_WARNING,
  NATIVE_CTA_PLS_METHOD_VERSION,
  NATIVE_CTA_PLS_RESULT_WARNING,
} from "./nativeCtaPls";

export function completedCtaPlsRun(): AnalysisRun {
  const scopeWarning = NATIVE_CTA_PLS_ESTIMATION_WARNING;
  return {
    id: "cta-run",
    modelId: "cta-model",
    name: "CTA-PLS descriptive tetrads",
    method: "Confirmatory Tetrad Analysis",
    createdAt: "2026-08-13T12:00:00Z",
    seed: 20_260_813,
    status: "completed",
    warnings: [scopeWarning],
    fingerprint: "sha256:cta",
    modelSnapshot: {
      nodes: [
        { id: "x", position: { x: 0, y: 0 }, data: { label: "Composite X", shortName: "X", mode: "formative", indicators: ["x1", "x2", "x3", "x4"] } },
        { id: "y", position: { x: 250, y: 0 }, data: { label: "Outcome Y", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
      ],
      edges: [{ id: "x-y", source: "x", target: "y" }],
    },
    result: {
      method_version: NATIVE_CTA_PLS_METHOD_VERSION,
      converged: true,
      iterations: 7,
      used_observations: 80,
      omitted_observations: 2,
      outer_estimates: ["x1", "x2", "x3", "x4"].map((indicator) => ({ construct: "x", indicator, weight: 0.25, loading: 0.7 })),
      paths: [{ source: "x", target: "y", coefficient: 0.5 }],
      effects: [{ source: "x", target: "y", direct: 0.5, indirect: 0, total: 0.5 }],
      cta_pls: {
        method_version: NATIVE_CTA_PLS_METHOD_VERSION,
        covariance: NATIVE_CTA_PLS_COVARIANCE_VERSION,
        estimates: [
          { construct: "x", indicator_a: "x1", indicator_b: "x2", indicator_c: "x3", indicator_d: "x4", pairing: "ab_cd_minus_ac_bd", tetrad: 0.01, absolute_tetrad: 0.01 },
          { construct: "x", indicator_a: "x1", indicator_b: "x2", indicator_c: "x3", indicator_d: "x4", pairing: "ac_bd_minus_ad_bc", tetrad: -0.004, absolute_tetrad: 0.004 },
          { construct: "x", indicator_a: "x1", indicator_b: "x2", indicator_c: "x3", indicator_d: "x4", pairing: "ad_bc_minus_ab_cd", tetrad: -0.006, absolute_tetrad: 0.006 },
        ],
        max_absolute_tetrad_by_construct: { x: 0.01 },
        warnings: [NATIVE_CTA_PLS_RESULT_WARNING],
      },
      r_squared: { y: 0.25 },
      warnings: [scopeWarning],
    },
    provenance: {
      recipe_id: "00000000-0000-4000-8000-000000000001",
      dataset_fingerprint: "sha256:cta",
      method: "cta_pls",
      method_version: `pls_pm_v1+${NATIVE_CTA_PLS_METHOD_VERSION}+pls_mediation_v1+assessment_v7`,
      engine_version: "quickpls-engine-test",
      seed: 20_260_813,
      settings: {
        method: "cta_pls",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: 20_260_813,
        workers: 1,
        confidence_level: 0.95,
        preprocessing: "standardized",
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-13T11:59:59Z",
      completed_at: "2026-08-13T12:00:00Z",
    },
  };
}
