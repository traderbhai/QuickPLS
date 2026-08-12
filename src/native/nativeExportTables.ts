import type { ResultTable } from "../domain/resultTables";
import type { AnalysisRun } from "../types";
import {
  CURRENT_CVPAT_METHOD_VERSION,
  CURRENT_PLS_PREDICT_METHOD_VERSION,
  CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION,
} from "./nativeCalculationMode";
import {
  nativeNcaCeilingLabel,
  nativeNcaResultProjection,
  nativeCbsemResultProjection,
  nativeGscaResultProjection,
  nativePcaComponentRuleLabel,
  nativePcaResultProjection,
  nativeOlsResultProjection,
  nativeLogisticResultProjection,
  nativeLegacyLogisticResultProjection,
  nativeRegressionBootstrapResultProjection,
} from "./nativeResults";

export interface NativeRunSettingApplicability {
  usesSeed: boolean;
  usesConfidenceLevel: boolean;
  usesWorkers: boolean;
}

export function nativeRunSettingApplicability(run: AnalysisRun): NativeRunSettingApplicability {
  const settings = run.provenance?.settings;
  const usesBootstrap = (settings?.bootstrap_samples ?? 0) > 0 || Boolean(run.bootstrap);
  const usesPermutation = (settings?.permutation_samples ?? 0) > 0 || Boolean(run.permutation);
  const usesGroupPermutation = run.provenance?.method === "mga" && Boolean(run.result?.mga_permutation);
  const usesNcaPermutation = run.provenance?.method === "nca" && run.result?.nca?.method_version === "nca_v2";
  const usesPredictionResampling = currentPredictionResult(run) !== null;
  const usesOlsConfidence = nativeOlsResultProjection(run) !== null;
  const usesLogisticConfidence = nativeLogisticResultProjection(run) !== null
    || nativeLegacyLogisticResultProjection(run) !== null;
  return {
    usesSeed: usesBootstrap || usesPermutation || usesGroupPermutation || usesNcaPermutation || usesPredictionResampling,
    usesConfidenceLevel: usesBootstrap || usesGroupPermutation || usesPredictionResampling || usesOlsConfidence || usesLogisticConfidence,
    usesWorkers: usesBootstrap || usesPermutation,
  };
}

export function nativeRunProvenanceTable(
  run: AnalysisRun,
  status: ResultTable["status"] = "validated",
): ResultTable {
  const applicability = nativeRunSettingApplicability(run);
  const nca = nativeNcaResultProjection(run);
  const pca = nativePcaResultProjection(run);
  const ols = nativeOlsResultProjection(run);
  const logistic = nativeLogisticResultProjection(run);
  const legacyLogistic = nativeLegacyLogisticResultProjection(run);
  const regressionBootstrap = nativeRegressionBootstrapResultProjection(run);
  const cbsem = nativeCbsemResultProjection(run);
  const gsca = nativeGscaResultProjection(run);
  const prediction = currentPredictionResult(run);
  const rows: string[][] = [
    ["Run", run.name],
    ["Method", run.method],
    ["Completed at", run.createdAt],
    ["Dataset fingerprint", run.provenance?.dataset_fingerprint ?? run.fingerprint],
  ];
  if (applicability.usesSeed) rows.splice(3, 0, ["Seed", String(run.provenance?.seed ?? run.seed)]);
  if (run.provenance) {
    rows.push(
      ["Recipe", run.provenance.recipe_id],
      ["Engine version", run.provenance.engine_version],
      ["Method version", run.provenance.method_version],
    );
    if (nca) {
      rows.push(
        ["Condition variable (X)", nca.x],
        ["Outcome variable (Y)", nca.y],
        ["Analyzed observations", String(nca.observations)],
        ["Ceiling lines", nativeNcaCeilingLabel(nca.ceiling)],
        ["Requested NCA permutations", String(nca.permutationSamples)],
        ["Usable NCA permutations", String(nca.usablePermutations)],
        ["Preprocessing", "unstandardized"],
        ["Missing data", "listwise deletion"],
      );
    } else if (pca) {
      rows.push(
        ["Selected variables", pca.variables.join(", ")],
        ["Analyzed observations", String(pca.observations)],
        ["Retention rule", nativePcaComponentRuleLabel(pca.componentRule)],
        ["Retained components", String(pca.retainedComponents)],
        ["Input matrix", "Correlation matrix of standardized variables"],
        ["Missing data", "Listwise deletion"],
        ["Maximum eigensolver iterations", String(run.provenance.settings.max_iterations)],
        ["Eigensolver stop criterion", String(run.provenance.settings.tolerance)],
      );
    } else if (ols) {
      rows.push(
        ["Outcome", ols.outcome],
        ["Predictors", ols.predictors.join(", ")],
        ["Controls", ols.controls.length ? ols.controls.join(", ") : "None"],
        ["Analyzed observations", String(ols.observations)],
        ["Estimator", "Ordinary least squares with intercept"],
        ["Standard errors", "HC3 heteroskedasticity-consistent"],
        ["Preprocessing", "unstandardized"],
        ["Missing data", "listwise deletion"],
      );
    } else if (logistic) {
      rows.push(
        ["Outcome", logistic.outcome],
        ["Predictors", logistic.predictors.join(", ")],
        ["Controls", logistic.controls.length ? logistic.controls.join(", ") : "None"],
        ["Analyzed observations", String(logistic.observations)],
        ["Estimator", "Binary logistic maximum likelihood with intercept"],
        ["Algorithm", "Deterministic Newton IRLS"],
        ["Converged", "Yes"],
        ["Optimizer iterations", String(logistic.diagnostics.convergence.iterations)],
        ["Outcome coding", "Numeric 0/1 (exact)"],
        ["Classification threshold", "0.5"],
        ["Coefficient inference", "Maximum-likelihood SE; Wald z; two-sided 95% confidence intervals"],
        ["Pseudo-R-squared", "McFadden"],
        ["Preprocessing", "unstandardized"],
        ["Missing data", "listwise deletion"],
      );
    } else if (legacyLogistic) {
      rows.push(
        ["Historical result", "Legacy binary logistic regression (v1)"],
        ["Outcome", legacyLogistic.outcome],
        ["Predictors", legacyLogistic.predictors.join(", ")],
        ["Controls", legacyLogistic.controls.length ? legacyLogistic.controls.join(", ") : "None"],
        ["Analyzed observations", String(legacyLogistic.observations)],
        ["Version handling", "Readable and exportable under its original version; not reinterpreted as v2 evidence"],
        ["Recorded historical preprocessing", legacyLogistic.recordedPreprocessing],
        ["Historical preprocessing handling", "Recorded for archive provenance only; non-operative for this preserved v1 result"],
        ["Missing data", "listwise deletion"],
      );
    } else if (cbsem) {
      rows.push(
        ["Model type", cbsem.modelType === "cfa" ? "Confirmatory factor analysis" : "Recursive structural equation model"],
        ["Estimator", "Maximum likelihood"],
        ["Input", "Raw case-level data"],
        ["Analyzed observations", String(cbsem.analysis.sample_size)],
        ["Preprocessing", "Indicators standardized after listwise filtering"],
        ["Missing data", "Listwise deletion"],
        ["Identification", "First loading fixed to 1 for each latent factor"],
        ["Mean structure", "Not estimated"],
        ["Converged", cbsem.analysis.converged ? "Yes" : "No"],
        ["Optimizer iterations", String(cbsem.analysis.iterations)],
        ["Maximum optimizer iterations", String(run.provenance.settings.max_iterations)],
        ["Optimizer stop criterion", String(run.provenance.settings.tolerance)],
        ["Estimator method version", cbsem.methodVersion],
        ["Fit method version", cbsem.analysis.fit.method_version],
        ["Modification-diagnostic version", "cbsem_modification_indices_v1"],
      );
    } else if (gsca) {
      rows.push(
        ["Estimator", "Joint global least-squares alternating least squares"],
        ["Algorithm version", gsca.algorithmVersion],
        ["Analyzed observations", String(gsca.usedObservations)],
        ["Omitted observations", String(gsca.omittedObservations)],
        ["Converged", gsca.analysis.converged ? "Yes" : "No"],
        ["ALS iterations", String(gsca.analysis.iterations)],
        ["Objective", String(gsca.analysis.objective)],
        ["Global FIT", String(gsca.analysis.fit)],
        ["Adjusted FIT", String(gsca.analysis.adjusted_fit)],
        ["Measurement FIT", String(gsca.analysis.measurement_fit)],
        ["Structural FIT", String(gsca.analysis.structural_fit)],
        ["GFI", String(gsca.analysis.gfi)],
        ["SRMR", String(gsca.analysis.srmr)],
        ["Initialization", "Deterministic +1 block weights"],
        ["Input", "Raw case-level data with listwise-standardized numeric indicators"],
        ["Missing data", "Listwise deletion"],
        ["Maximum ALS iterations", "3000"],
        ["ALS stop criterion", "1e-7 for both objective and normalized weights"],
        ["Inference", "Point estimates only"],
      );
    } else {
      rows.push(
        ["Weighting scheme", run.provenance.settings.weighting_scheme],
        ["Preprocessing", run.provenance.settings.preprocessing],
        ["Maximum iterations", String(run.provenance.settings.max_iterations)],
        ["Stop criterion", String(run.provenance.settings.tolerance)],
      );
    }
    if (regressionBootstrap) {
      rows.push(
        ["Regression bootstrap method", regressionBootstrap.method_version],
        ["Regression bootstrap sampling", "Case resampling with replacement"],
        ["Regression bootstrap algorithm", regressionBootstrap.algorithm],
        ["Regression bootstrap stream", regressionBootstrap.stream_token],
        ["Regression bootstrap alternative", "Two-sided"],
        ["Regression bootstrap test reference", regressionBootstrap.test_reference],
        ["Regression bootstrap test tolerance policy", regressionBootstrap.test_tolerance_policy],
        ["Regression bootstrap interval policy", "Percentile primary; BCa conditional"],
        ["Requested regression bootstrap replicates", String(regressionBootstrap.requested_replicates)],
        ["Usable regression bootstrap replicates", String(regressionBootstrap.usable_replicates)],
        ["Failed regression bootstrap replicates", String(regressionBootstrap.failed_replicates.length)],
        ["Regression bootstrap delete-one fits required", String(regressionBootstrap.jackknife_cases)],
        ["Regression bootstrap delete-one fits usable", String(regressionBootstrap.usable_jackknife_cases)],
        ["Regression bootstrap workers", String(regressionBootstrap.workers)],
        ["Regression bootstrap reproducibility", "Fixed seed with deterministic worker-invariant indexed streams"],
      );
    }
    if (applicability.usesConfidenceLevel) {
      rows.push([
        prediction ? "CVPAT confidence level" : "Confidence level",
        prediction ? "0.95" : String(run.provenance.settings.confidence_level),
      ]);
    }
    if (applicability.usesWorkers) {
      rows.push(["Workers", String(run.provenance.settings.workers)]);
    }
    if (run.provenance.settings.bootstrap_samples > 0) {
      rows.push(["Bootstrap samples", String(run.provenance.settings.bootstrap_samples)]);
    }
    if (run.provenance.settings.permutation_samples > 0) {
      rows.push(["Permutation samples", String(run.provenance.settings.permutation_samples)]);
    }
    if (prediction) {
      rows.push(
        ["Prediction scope", "Endogenous indicators; construct scores supplementary"],
        ["Repeated prediction method version", prediction.repeated.method_version],
        ["CVPAT method version", CURRENT_CVPAT_METHOD_VERSION],
        ["Primary validation", `${prediction.repeated.folds}-fold × ${prediction.repeated.repeats}-repeat cross-validation`],
        ["Fold assignment", "Seeded balanced fold assignment"],
        ["Assignment digest", prediction.repeated.assignment_digest ?? ""],
        ["Primary test predictions", String(prediction.repeated.total_test_observations)],
        ["Secondary holdout", `${prediction.predict.training_observations} training / ${prediction.predict.test_observations} test observations`],
        ["Benchmarks", "Indicator average (IA); Linear model (LM, where estimable)"],
        ["CVPAT alternative", "PLS-SEM loss < benchmark (one-sided)"],
        ["Missing data", "Listwise deletion across all model indicators"],
      );
    }
    const micom = run.result?.micom;
    if (applicability.usesSeed && run.provenance.method === "mga" && micom?.method_version === "micom_v2") {
      rows.push(
        ["MICOM Step 1", "Researcher confirmed configural invariance"],
        ["Requested group permutations", String(micom.permutation_samples)],
        ["Usable group permutations", String(micom.usable_permutations)],
      );
    }
    if (run.provenance.settings.case_weight_column) {
      rows.push(["Case-weight variable", run.provenance.settings.case_weight_column]);
    }
  } else if (run.result?.method_version) {
    rows.push(["Method version", run.result.method_version]);
  }
  if (run.warnings.length > 0) {
    rows.push(["Warnings", run.warnings.join("; ")]);
  }
  return {
    id: "run_provenance",
    title: "Run provenance",
    status,
    warning: null,
    columns: ["Field", "Value"],
    rows,
  };
}

export function nativePcaScoreExportTable(run: AnalysisRun): ResultTable | null {
  const projection = nativePcaResultProjection(run);
  const scores = run.result?.pca?.scores;
  if (!projection || !scores) return null;
  const byComponent = new Map(projection.components.map((component) => [component.component, Array<number>(projection.observations)]));
  for (const row of scores) {
    const values = byComponent.get(row.component);
    if (!values || row.observation < 0 || row.observation >= projection.observations || !Number.isFinite(row.score)) return null;
    values[row.observation] = row.score;
  }
  if ([...byComponent.values()].some((values) => values.some((value) => !Number.isFinite(value)))) return null;
  return {
    id: "pca_scores",
    title: "Component scores",
    status: "validated",
    warning: null,
    columns: ["Complete-case observation", ...projection.components.map((component) => component.component)],
    rows: Array.from({ length: projection.observations }, (_, observation) => [
      String(observation + 1),
      ...projection.components.map((component) => {
        const value = byComponent.get(component.component)?.[observation];
        return Number.isFinite(value) ? (value as number).toFixed(6).replace(/^-0\.000000$/, "0.000000") : "";
      }),
    ]),
  };
}

export function nativeOlsPredictionExportTable(run: AnalysisRun): ResultTable | null {
  const projection = nativeOlsResultProjection(run);
  const predictions = run.result?.regression?.predictions;
  if (!projection || !predictions || predictions.length !== projection.observations) return null;
  if (predictions.some((row, index) => row.observation !== index
    || !Number.isFinite(row.fitted)
    || !Number.isFinite(row.residual)
    || row.probability != null)) return null;
  const format = (value: number) => value.toFixed(6).replace(/^-0\.000000$/, "0.000000");
  return {
    id: "ols_fitted_residuals",
    title: "Fitted values and residuals",
    status: "validated",
    warning: null,
    columns: ["Complete-case observation", "Fitted", "Residual"],
    rows: predictions.map((row) => [
      String(row.observation + 1),
      format(row.fitted),
      format(row.residual!),
    ]),
  };
}

function currentPredictionResult(run: AnalysisRun) {
  const predict = run.result?.predict;
  const repeated = predict?.repeated_kfold;
  const assessments = repeated?.cvpat_benchmark_assessments ?? [];
  if (
    predict?.method_version !== CURRENT_PLS_PREDICT_METHOD_VERSION
    || repeated?.method_version !== CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION
    || !/^sha256:[0-9a-f]{64}$/.test(repeated.assignment_digest ?? "")
    || assessments.length !== 2
    || new Set(assessments.map((row) => row.benchmark)).size !== 2
    || !assessments.every((row) => row.method_version === CURRENT_CVPAT_METHOD_VERSION)
  ) return null;
  return { predict, repeated };
}
