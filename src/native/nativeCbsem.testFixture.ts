import { completedSamplePlsRun } from "../data/smokeRun";
import type { AnalysisRun, CbsemAnalysis, CbsemMatrixCell } from "../types";

const indicatorOrder = ["x1", "x2", "y1", "y2"] as const;

function covarianceCells(): CbsemMatrixCell[] {
  return indicatorOrder.flatMap((row, rowIndex) => indicatorOrder.map((column, columnIndex) => ({
    row,
    column,
    value: rowIndex === columnIndex ? 1 : 0.2,
  })));
}

export function completedCbsemRun(modelType: "cfa" | "sem" = "sem"): AnalysisRun {
  const base = completedSamplePlsRun();
  const methodVersion = modelType === "cfa" ? "cfa_ml_v1" : "cbsem_ml_v1";
  const parameters: CbsemAnalysis["parameters"] = [
    { name: "loading:x:x1", kind: "loading", lhs: "x", rhs: "x1", estimate: 1, standard_error: null, z_statistic: null, p_value_two_sided: null, fixed: true },
    { name: "loading:x:x2", kind: "loading", lhs: "x", rhs: "x2", estimate: 0.84, standard_error: 0.05, z_statistic: 16.8, p_value_two_sided: 0.0001, fixed: false },
    { name: "loading:y:y1", kind: "loading", lhs: "y", rhs: "y1", estimate: 1, standard_error: null, z_statistic: null, p_value_two_sided: null, fixed: true },
    { name: "loading:y:y2", kind: "loading", lhs: "y", rhs: "y2", estimate: 0.9, standard_error: 0.04, z_statistic: 22.5, p_value_two_sided: 0.0001, fixed: false },
    ...(modelType === "sem" ? [{ name: "path:y:x", kind: "structural_path", lhs: "y", rhs: "x", estimate: 0.61, standard_error: 0.08, z_statistic: 7.625, p_value_two_sided: 0.0001, fixed: false }] : []),
    { name: "variance:x:x", kind: "latent_variance", lhs: "x", rhs: "x", estimate: 0.95, standard_error: 0.1, z_statistic: 9.5, p_value_two_sided: 0.0001, fixed: false },
    { name: "variance:y:y", kind: "latent_variance", lhs: "y", rhs: "y", estimate: 0.58, standard_error: 0.08, z_statistic: 7.25, p_value_two_sided: 0.0001, fixed: false },
  ];
  const standardized: CbsemAnalysis["standardized"] = parameters.map((parameter) => ({
    name: parameter.name,
    kind: parameter.kind,
    lhs: parameter.lhs,
    rhs: parameter.rhs,
    std_lv: parameter.kind === "structural_path" ? 0.56 : parameter.kind === "latent_variance" && parameter.lhs === "y" ? 0.64 : parameter.estimate,
    std_all: parameter.kind === "structural_path" ? 0.56 : parameter.kind === "latent_variance" && parameter.lhs === "y" ? 0.64 : parameter.estimate,
  }));
  const impliedCovariance = covarianceCells();
  const residualCovariance = impliedCovariance.map((cell) => ({ ...cell, value: cell.row === cell.column ? 0.04 : 0.01 }));
  const residualCorrelation = residualCovariance.map((cell) => ({ ...cell, value: cell.row === cell.column ? 0 : 0.025 }));
  const analysis: CbsemAnalysis = {
    method_version: methodVersion,
    model_type: modelType,
    estimator: "ml",
    input: "raw",
    mean_structure: false,
    converged: true,
    iterations: 24,
    objective: 0.114,
    gradient_norm: 0.0000004,
    sample_size: 120,
    parameters,
    standardized,
    implied_covariance: impliedCovariance,
    residual_covariance: residualCovariance,
    residual_correlation: residualCorrelation,
    fit: {
      method_version: "cbsem_fit_v1",
      chi_square: 12.4,
      degrees_of_freedom: 5,
      p_value: 0.0296,
      cfi: 0.982,
      tli: 0.973,
      rmsea: 0.072,
      rmsea_ci_lower: 0.021,
      rmsea_ci_upper: 0.121,
      srmr: 0.031,
      aic: 1012.4,
      bic: 1054.2,
      baseline_chi_square: 218.7,
      baseline_degrees_of_freedom: 6,
    },
    modification_indices: [{
      method_version: "cbsem_modification_indices_v1",
      kind: "residual_covariance",
      lhs: "x2",
      rhs: "y2",
      modification_index: 4.12,
      expected_parameter_change: 0.09,
    }],
    diagnostics: ["Single-group ML converged."],
    warnings: ["Residual-based modification diagnostics are screening diagnostics only."],
  };
  const assessmentVersion = base.assessment!.method_version;

  return {
    ...base,
    id: `cbsem-${modelType}-run`,
    name: modelType === "cfa" ? "Confirmatory factor analysis run" : "CB-SEM run",
    method: "CB-SEM / CFA",
    modelSnapshot: {
      nodes: [
        { id: "x", type: "construct", position: { x: 50, y: 80 }, data: { label: "Latent Predictor", shortName: "X", mode: "reflective", indicators: ["x1", "x2"] } },
        { id: "y", type: "construct", position: { x: 350, y: 80 }, data: { label: "Latent Outcome", shortName: "Y", mode: "reflective", indicators: ["y1", "y2"] } },
      ],
      edges: modelType === "sem" ? [{ id: "x-y", source: "x", target: "y" }] : [],
    },
    bootstrap: undefined,
    permutation: undefined,
    provenance: {
      recipe_id: "recipe-cbsem-fixture",
      dataset_fingerprint: "sha256:cbsem-fixture",
      method: "cbsem",
      method_version: `pls_pm_v1+${methodVersion}+cbsem_fit_v1+cbsem_modification_indices_v1+pls_mediation_v1+${assessmentVersion}`,
      engine_version: "2.45.0",
      seed: base.seed,
      settings: {
        method: "cbsem",
        weighting_scheme: "path",
        tolerance: 1e-7,
        max_iterations: 3_000,
        preprocessing: "standardized",
        bootstrap_samples: 0,
        studentized_inner_samples: 0,
        permutation_samples: 0,
        seed: base.seed,
        workers: 1,
        confidence_level: 0.95,
        missing_data: "listwise_deletion",
        case_weight_column: null,
      },
      started_at: "2026-08-12T03:00:00.000Z",
      completed_at: "2026-08-12T03:00:01.000Z",
    },
    result: {
      ...base.result!,
      method_version: methodVersion,
      iterations: 24,
      used_observations: 120,
      paths: modelType === "sem" ? [{ source: "x", target: "y", coefficient: 0.1 }] : [],
      outer_estimates: [
        { construct: "x", indicator: "x1", loading: 0.1, weight: 0.1 },
        { construct: "y", indicator: "y1", loading: 0.1, weight: 0.1 },
      ],
      cbsem: analysis,
    },
  };
}
