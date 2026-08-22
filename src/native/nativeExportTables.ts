import type { ResultTable } from "../domain/resultTables";
import type { AnalysisRun } from "../types";
import {
  CURRENT_CVPAT_METHOD_VERSION,
  CURRENT_PLS_PREDICT_METHOD_VERSION,
  CURRENT_PLS_PREDICT_REPEATED_METHOD_VERSION,
} from "./nativeCalculationMode";
import {
  CBSEM_RMSEA_INTERVAL_METHOD_VERSION_V1,
  nativeNcaCeilingLabel,
  nativeNcaCeFdhPeerTable,
  nativeNcaResultProjection,
  nativeCbsemResultProjection,
  nativeCtaPlsResultProjection,
  nativeGscaResultProjection,
  nativePcaComponentRuleLabel,
  nativePcaResultProjection,
  nativePcaScoreResultTable,
  nativeOlsResultProjection,
  nativeLogisticResultProjection,
  nativeLegacyLogisticResultProjection,
  nativeRegressionBootstrapResultProjection,
  nativeProcessResultProjection,
  nativeModelFitPresentationStateV2,
  nativePlsModelFitExactProjection,
  nativePlsModelFitV2Projection,
} from "./nativeResults";
import {
  NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING,
  nativeStructuralPathRandomizationProjection,
} from "./nativeStructuralPathRandomization";
import { nativePlscConsistentBootstrapProjection } from "./nativeConsistentBootstrap";
import { nativePlscConsistentPermutationProjection } from "./nativeConsistentPermutation";

export interface NativeRunSettingApplicability {
  usesSeed: boolean;
  usesConfidenceLevel: boolean;
  usesWorkers: boolean;
}

/** Rebuilds the CE-FDH peer export from the immutable selected NCA run. */
export function nativeNcaCeFdhPeerExportTable(run: AnalysisRun): ResultTable | null {
  return nativeNcaCeFdhPeerTable(run);
}

export function nativeRunSettingApplicability(run: AnalysisRun): NativeRunSettingApplicability {
  const settings = run.provenance?.settings;
  const usesBootstrap = (settings?.bootstrap_samples ?? 0) > 0 || Boolean(run.bootstrap);
  const usesPermutation = nativeStructuralPathRandomizationProjection(run) !== null;
  const usesConsistentPermutation = nativePlscConsistentPermutationProjection(run) !== null;
  const usesGroupPermutation = run.provenance?.method === "mga" && Boolean(run.result?.mga_permutation);
  const usesNcaPermutation = run.provenance?.method === "nca" && run.result?.nca?.method_version === "nca_v2";
  const usesPredictionResampling = currentPredictionResult(run) !== null;
  const usesProcess = nativeProcessResultProjection(run) !== null;
  const usesOlsConfidence = nativeOlsResultProjection(run) !== null;
  const usesLogisticConfidence = nativeLogisticResultProjection(run) !== null
    || nativeLegacyLogisticResultProjection(run) !== null;
  const usesProspectivePower = run.provenance?.method === "pls_sample_size_power"
    && Boolean(run.plsSampleSizePower && run.plsSampleSizePowerRecipe);
  const cbsemProjection = nativeCbsemResultProjection(run);
  const usesCbsemBootstrap = cbsemProjection?.analysis.bootstrap_v2 != null
    || cbsemProjection?.analysis.exact_case_bootstrap != null
    || cbsemProjection?.analysis.exact_case_bootstrap_studentized != null
    || cbsemProjection?.analysis.exact_case_bootstrap_bca != null;
  return {
    usesSeed: usesBootstrap || usesPermutation || usesConsistentPermutation || usesGroupPermutation || usesNcaPermutation || usesPredictionResampling || usesProspectivePower || usesCbsemBootstrap,
    usesConfidenceLevel: usesBootstrap || usesConsistentPermutation || usesGroupPermutation || usesPredictionResampling || usesOlsConfidence || usesLogisticConfidence || usesProcess || usesProspectivePower || usesCbsemBootstrap,
    usesWorkers: usesBootstrap || usesPermutation || usesConsistentPermutation || usesProspectivePower || usesCbsemBootstrap,
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
  const process = nativeProcessResultProjection(run);
  const cbsem = nativeCbsemResultProjection(run);
  const gsca = nativeGscaResultProjection(run);
  const ctaPls = nativeCtaPlsResultProjection(run);
  const structuralPathRandomization = nativeStructuralPathRandomizationProjection(run);
  const consistentBootstrap = nativePlscConsistentBootstrapProjection(run);
  const ordinaryPlsBootstrap = run.bootstrap && !consistentBootstrap ? run.bootstrap : null;
  const consistentPermutation = nativePlscConsistentPermutationProjection(run);
  const htmtInference = currentHtmtBootstrapInference(run);
  const plsModelFit = nativePlsModelFitV2Projection(run);
  const plsModelFitExact = nativePlsModelFitExactProjection(run);
  const plsModelFitPresentation = nativeModelFitPresentationStateV2(run);
  const effectiveStatus = consistentPermutation || cbsem?.analysis.bootstrap_v2
    || cbsem?.analysis.exact_case_bootstrap || cbsem?.analysis.exact_case_bootstrap_studentized
    || cbsem?.analysis.exact_case_bootstrap_bca
    || htmtInference || plsModelFitExact
    ? "experimental"
    : status;
  const prediction = currentPredictionResult(run);
  const rows: string[][] = [
    ["Run", run.name],
    ["Method", run.method],
    ["Completed at", run.createdAt],
    ["Dataset fingerprint", run.provenance?.dataset_fingerprint ?? run.fingerprint],
  ];
  if (applicability.usesSeed) rows.splice(3, 0, ["Seed", String(run.provenance?.seed ?? run.seed)]);
  if (plsModelFit) {
    rows.push(
      ["PLS model-fit method version", plsModelFit.method_version!],
      ["PLS model-fit analytical observations", String(plsModelFit.analytical_sample_size)],
      ["PLS model-fit d_G logarithm", "Natural logarithm"],
      ["PLS model-fit exact-fit procedure", "Adapted Bollen-Stine for saturated and estimated models"],
      ["PLS model-fit exact-fit inference", plsModelFitPresentation?.detailValue ?? "Not run"],
      ...(plsModelFitPresentation ? [[
        "PLS model-fit interpretation",
        plsModelFitPresentation.advisory.message,
      ]] : []),
    );
    if (plsModelFitExact) {
      rows.push(
        ["PLS model-fit exact method version", plsModelFitExact.method_version],
        ["PLS model-fit exact requested replicates per model", String(plsModelFitExact.requested_replicates)],
        ["PLS model-fit exact master seed", String(plsModelFitExact.master_seed)],
        ["PLS model-fit exact retry policy", "No retry or replacement; fixed indexed draws"],
        ["PLS model-fit saturated exact status", plsModelFitExact.saturated.status],
        ["PLS model-fit estimated exact status", plsModelFitExact.estimated.status],
      );
    }
  } else if (plsModelFitPresentation?.mode === "higher_order_not_reported") {
    rows.push(
      ["PLS model-fit reporting", plsModelFitPresentation.detailValue],
      ["PLS model-fit interpretation", plsModelFitPresentation.advisory.message],
    );
  }
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
    } else if (run.plsSampleSizePower && run.plsSampleSizePowerRecipe) {
      const power = run.plsSampleSizePower;
      const recipe = run.plsSampleSizePowerRecipe;
      rows.push(
        ["Capability", power.capability_id],
        ["Prospective method", power.method_version],
        ["Scenario", recipe.scenario_identity],
        ["Target path", `${recipe.design.predictor_construct} -> ${recipe.design.outcome_construct}`],
        ["Population path", String(recipe.design.population_path)],
        ["Sample-size grid", recipe.sample_size_grid.join(", ")],
        ["Monte Carlo replicates per grid point", String(recipe.monte_carlo_replicates)],
        ["Case-bootstrap replicates per dataset", String(recipe.bootstrap_replicates)],
        ["Planned PLS fits", String(power.workload.estimated_pls_fits)],
        ["Planned fitted rows", String(power.workload.estimated_pls_case_fits)],
        ["Grid decision rule", "First evaluated n whose Wilson lower bound reaches target; no interpolation or extrapolation"],
        ["Monotonicity violations", String(power.monotonicity_violations)],
        ["Recipe digest", power.recipe_digest],
        ["Outcome digest", power.outcome_digest],
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
    } else if (ctaPls) {
      rows.push(
        ["Diagnostic", "CTA-PLS descriptive sample-covariance tetrads"],
        ["Eligible blocks", String(ctaPls.blocks.length)],
        ["Reported tetrads", String(ctaPls.estimates.length)],
        ["Analyzed observations", String(ctaPls.usedObservations)],
        ["Omitted observations", String(ctaPls.omittedObservations)],
        ["Covariance convention", ctaPls.covarianceVersion],
        ["Inference", "Not calculated; bootstrap, permutation, asymptotic, and vanishing-tetrad decisions are excluded"],
        ["Missing data", "Listwise deletion across model indicators"],
      );
    } else if (process) {
      rows.push(
        ["Outcome", process.outcome],
        ["Predictors in canonical graph order", process.predictors.join(", ")],
        ["Controls in every equation", process.controls.length ? process.controls.join(", ") : "None"],
        ["Global complete cases", String(process.observations)],
        ["Rows omitted listwise", String(process.omittedObservations)],
        ["Directed paths", String(process.graph.paths.length)],
        ["Moderated paths", String(process.graph.moderations.length)],
        ["OLS equations", String(process.graph.equations.length)],
        ["Estimator", "Raw observed-variable OLS equations with intercept"],
        ["Covariance", "HC3"],
        ["Coefficient inference", "Student-t residual df; fixed two-sided 95% confidence intervals"],
        ["Continuous product centering", "Equation complete-case mean within each original, resample, or delete-one fit"],
        ["Moderator probes", "Original-sample raw mean - SD, mean, mean + SD for continuous moderators; raw 0/1 for binary moderators"],
        ["Plots", "Persisted engine-produced raw points and confidence intervals; no UI scientific recomputation"],
        ["Missing data", "One global listwise-complete sample"],
      );
      if (process.bootstrap) rows.push(
        ["PROCESS bootstrap method", process.bootstrap.method_version],
        ["PROCESS bootstrap sampling", "Indexed complete-case resampling with replacement"],
        ["PROCESS bootstrap stream", process.bootstrap.stream_token],
        ["PROCESS bootstrap interval policy", "Percentile primary; BCa conditional on every delete-one fit"],
        ["PROCESS bootstrap test reference", "Two-sided standard-normal bootstrap ratio"],
        ["Requested PROCESS bootstrap replicates", String(process.bootstrap.requested_replicates)],
        ["Usable PROCESS bootstrap replicates", String(process.bootstrap.usable_replicates)],
        ["Failed PROCESS bootstrap replicates", String(process.bootstrap.failed_replicates.length)],
        ["PROCESS delete-one fits usable / required", `${process.bootstrap.usable_jackknife_cases} / ${process.bootstrap.jackknife_cases}`],
        ["PROCESS bootstrap workers", String(process.bootstrap.workers)],
        ["PROCESS bootstrap reproducibility", "Fixed seed with deterministic worker-invariant indexed streams and original raw probe grid"],
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
        ["Version handling", "Readable and exportable under its original version; not reinterpreted as v2 output"],
        ["Recorded historical preprocessing", legacyLogistic.recordedPreprocessing],
        ["Historical preprocessing handling", "Recorded for archive provenance only; non-operative for this preserved v1 result"],
        ["Missing data", "listwise deletion"],
      );
    } else if (cbsem) {
      const cbsemBootstrap = cbsem.analysis.bootstrap_v2;
      const studentizedCbsemBootstrap = cbsem.analysis.exact_case_bootstrap_studentized;
      const bcaCbsemBootstrap = cbsem.analysis.exact_case_bootstrap_bca;
      const exactCbsemBootstrap = cbsem.analysis.exact_case_bootstrap
        ?? studentizedCbsemBootstrap?.base ?? bcaCbsemBootstrap?.base ?? null;
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
        ...(!cbsem.analysis.score_lm ? [["Modification-diagnostic version", "cbsem_modification_indices_v1"]] : []),
      );
      const rmseaInterval = cbsem.analysis.fit.rmsea_interval_attribution;
      if (rmseaInterval?.method_version === CBSEM_RMSEA_INTERVAL_METHOD_VERSION_V1) rows.push(
        ["RMSEA interval method", "Noncentral chi-square inversion (N - 1 denominator)"],
        ["RMSEA interval method version", rmseaInterval.method_version],
        ["RMSEA interval confidence", `${(rmseaInterval.confidence_level * 100).toFixed(1)}%`],
        ["RMSEA interval lower bound", cbsem.analysis.fit.rmsea_ci_lower == null ? "Not estimated" : String(cbsem.analysis.fit.rmsea_ci_lower === 0 ? 0 : cbsem.analysis.fit.rmsea_ci_lower)],
        ["RMSEA interval upper bound", cbsem.analysis.fit.rmsea_ci_upper == null ? "Not estimated" : String(cbsem.analysis.fit.rmsea_ci_upper === 0 ? 0 : cbsem.analysis.fit.rmsea_ci_upper)],
      );
      const scoreLm = cbsem.analysis.score_lm;
      if (scoreLm) rows.push(
        ["Score/LM method version", scoreLm.method_version],
        ["Score/LM scope", "Covariance-only CFA; explicitly declared zero residual covariances"],
        ["Score/LM candidate count", String(scoreLm.rows.length)],
        ["Score/LM available tests", String(scoreLm.rows.filter((row) => row.outcome.status === "available").length)],
        ["Score/LM unavailable tests", String(scoreLm.rows.filter((row) => row.outcome.status === "unavailable").length)],
      );
      if (cbsemBootstrap) rows.push(
        ["CB-SEM bootstrap method", cbsemBootstrap.method_version],
        ["CB-SEM bootstrap availability", "Experimental"],
        ["CB-SEM bootstrap sampling", "Raw complete-case resampling with replacement and a full production ML refit per preplanned draw"],
        ["CB-SEM bootstrap interval", "Percentile Type-7"],
        ["CB-SEM bootstrap confidence", `${(cbsemBootstrap.confidence_level * 100).toFixed(1)}%`],
        ["CB-SEM bootstrap inference", cbsemBootstrap.inference.status === "available" ? "Available" : "Unavailable - insufficient usable fits"],
        ["Requested CB-SEM bootstrap draws", String(cbsemBootstrap.requested_replicates)],
        ["Attempted CB-SEM ML fits", String(cbsemBootstrap.attempted_fits)],
        ["Usable CB-SEM ML fits", String(cbsemBootstrap.usable_replicates)],
        ["Failed CB-SEM ML fits", String(cbsemBootstrap.failed_replicates)],
        ["Minimum usable CB-SEM ML fits", String(cbsemBootstrap.minimum_usable_replicates)],
        ["CB-SEM bootstrap workers", String(run.provenance.settings.workers)],
        ["CB-SEM bootstrap failure policy", "No retry or replacement draw"],
      );
      if (exactCbsemBootstrap) rows.push(
        ["Exact CB-SEM bootstrap method", exactCbsemBootstrap.method_version],
        ["Exact CB-SEM bootstrap availability", "Experimental"],
        ["Exact CB-SEM bootstrap sampling", "Full exact-ML case bootstrap with preplanned primary draws"],
        ["Exact CB-SEM bootstrap interval", "Percentile Type-7; sample-SD standard errors"],
        ["Exact CB-SEM bootstrap confidence", `${(exactCbsemBootstrap.confidence_level * 100).toFixed(1)}%`],
        ["Exact CB-SEM bootstrap inference", exactCbsemBootstrap.inference.status === "available"
          ? "Available"
          : exactCbsemBootstrap.requested_replicates === 500
            ? "Unavailable — 500-draw pilot is below the frozen 1,000-usable-refit minimum"
            : "Unavailable — insufficient usable exact refits"],
        ["Requested exact CB-SEM refits", String(exactCbsemBootstrap.requested_replicates)],
        ["Usable exact CB-SEM refits", String(exactCbsemBootstrap.usable_replicates)],
        ["Failed exact CB-SEM refits", String(exactCbsemBootstrap.failed_replicates)],
        ["Exact CB-SEM bootstrap failure policy", "Failed fits retained; no retry or replacement draw"],
        ["Exact CB-SEM archive validation scope", "Schedule descriptors and arithmetic checked; raw fits and the Rust schedule were not replayed by the browser reader"],
      );
      const exactHypothesisTests = exactCbsemBootstrap?.hypothesis_tests;
      if (exactHypothesisTests) rows.push(
        ["Exact CB-SEM zero-null method", exactHypothesisTests.method_version],
        ["Exact CB-SEM zero-null selection", exactHypothesisTests.selected_test_tail === "two_sided"
          ? "Two-sided: parameter differs from zero"
          : exactHypothesisTests.selected_test_tail === "one_sided_greater"
            ? "One-sided: parameter is greater than zero"
            : "One-sided: parameter is less than zero"],
        ["Exact CB-SEM zero-null statistic", "Unstudentized null-centered parameter estimate"],
        ["Exact CB-SEM zero-null probability", "Inclusive tail count with (count + 1) / (usable + 1)"],
        ["Exact CB-SEM zero-null decision rule", "Selected p-value ≤ 0.05; no multiplicity adjustment"],
        ["Exact CB-SEM zero-null inference", exactHypothesisTests.inference.status === "available"
          ? "Available"
          : "Unavailable — insufficient usable exact refits"],
        ["Exact CB-SEM zero-null usable refits", String(exactHypothesisTests.usable_replicates)],
        ["Exact CB-SEM interval relationship", "The fixed two-sided 95% percentile interval is not reinterpreted by the selected test tail"],
      );
      if (studentizedCbsemBootstrap) {
        const sidecar = studentizedCbsemBootstrap.studentized;
        const pointOutcome = sidecar.point_standard_errors.outcome;
        const availableRefitStandardErrors = sidecar.refit_standard_errors.filter((receipt) => (
          receipt.outcome.status === "available"
        )).length;
        rows.push(
          ["Studentized CB-SEM method", sidecar.method_version],
          ["Studentized CB-SEM availability", "Experimental Labs"],
          ["Studentized CB-SEM standard-error method", sidecar.standard_error_method_version],
          ["Studentized CB-SEM expected-information method", sidecar.expected_information_method],
          ["Studentized CB-SEM pivot method", sidecar.pivot_method],
          ["Studentized CB-SEM quantile method", sidecar.quantile_method],
          ["Studentized CB-SEM interval method", sidecar.interval_method],
          ["Studentized CB-SEM confidence", `${(sidecar.confidence_level * 100).toFixed(1)}%`],
          ["Studentized CB-SEM minimum usable fraction", String(sidecar.minimum_usable_fraction)],
          ["Studentized CB-SEM minimum usable refits", String(sidecar.minimum_usable_replicates)],
          ["Studentized CB-SEM usable refits", String(sidecar.studentized_usable_replicates)],
          ["Studentized CB-SEM inference", sidecar.inference.status === "available"
            ? "Available"
            : `Unavailable — ${sidecar.inference.reason}: ${sidecar.inference.message}`],
          ["Studentized CB-SEM point standard errors", pointOutcome.status === "available"
            ? `Available for ${pointOutcome.parameters.length} parameter(s)`
            : `Unavailable — ${pointOutcome.reason}`],
          ["Studentized CB-SEM refit standard-error receipts", `${availableRefitStandardErrors} available; ${sidecar.refit_standard_errors.length - availableRefitStandardErrors} unavailable`],
          ["Studentized CB-SEM archive validation scope", sidecar.archive_validation_scope],
          ["Studentized CB-SEM archive reopening", "Ledger and arithmetic only; raw refits and expected-information calculations were not replayed"],
        );
      }
      if (bcaCbsemBootstrap) {
        const sidecar = bcaCbsemBootstrap.bca;
        rows.push(
          ["BCa CB-SEM method", sidecar.method_version],
          ["BCa CB-SEM availability", "Experimental Labs; complete-only delete-one inference"],
          ["BCa CB-SEM bias correction", sidecar.bias_correction_method],
          ["BCa CB-SEM acceleration", sidecar.acceleration_method],
          ["BCa CB-SEM adjusted probability", sidecar.adjusted_probability_method],
          ["BCa CB-SEM quantile", sidecar.quantile_method],
          ["BCa CB-SEM confidence", `${(sidecar.confidence_level * 100).toFixed(1)}%`],
          ["BCa CB-SEM bootstrap usable refits", String(sidecar.bootstrap_usable_replicates)],
          ["BCa CB-SEM minimum bootstrap usable refits", String(sidecar.minimum_bootstrap_usable_replicates)],
          ["BCa CB-SEM delete-one cases", String(sidecar.delete_one_case_count)],
          ["BCa CB-SEM successful delete-one refits", String(sidecar.successful_delete_one_refits.length)],
          ["BCa CB-SEM failed delete-one refits", String(sidecar.failed_delete_one_refits.length)],
          ["BCa CB-SEM inference", sidecar.inference.status === "available"
            ? "Available"
            : `Unavailable — ${sidecar.inference.reason}: ${sidecar.inference.message}`],
          ["BCa CB-SEM failure policy", "Exactly one fit per omitted complete case; any failure makes global BCa inference unavailable"],
          ["BCa CB-SEM archive validation scope", "Persisted ledger identity, digests, and exposed interval arithmetic only"],
          ["BCa CB-SEM archive reopening", "Raw base and delete-one ML fits were not replayed; Rust remains authoritative for fitting and BCa normal-probability transforms"],
        );
      }
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
    if (ordinaryPlsBootstrap) {
      rows.push(
        ["PLS bootstrap method", ordinaryPlsBootstrap.method_version],
        ["PLS bootstrap sampling", "Indexed complete-case resampling with replacement and one full PLS refit per preplanned draw"],
        ["PLS bootstrap interval policy", "Percentile primary; BCa conditional on delete-one fits; studentized conditional when requested"],
        ["Requested PLS bootstrap refits", String(ordinaryPlsBootstrap.plan.replicates)],
        ["Attempted PLS bootstrap refits", String(ordinaryPlsBootstrap.plan.replicates)],
        ["Usable PLS bootstrap refits", String(ordinaryPlsBootstrap.usable_replicates)],
        ["Failed PLS bootstrap refits", String(ordinaryPlsBootstrap.failed_replicates.length)],
        ["PLS bootstrap failure policy", "No retry or replacement draw"],
      );
    }
    if (consistentBootstrap) {
      rows.push(
        ["PLSc bootstrap method", consistentBootstrap.bootstrap.method_version],
        ["PLSc bootstrap estimator", consistentBootstrap.bootstrap.estimator_method_version!],
        ["PLSc bootstrap resampling kernel", consistentBootstrap.bootstrap.resampling_method_version!],
        ["PLSc bootstrap sampling", "Indexed complete-case resampling with replacement and a full PLSc refit per preplanned draw"],
        ["PLSc bootstrap interval policy", "Percentile primary; BCa conditional on every full-PLSc delete-one fit"],
        ["PLSc bootstrap test reference", "Two-sided standard-normal bootstrap ratio"],
        ["Requested PLSc bootstrap refits", String(consistentBootstrap.requestedReplicates)],
        ["Attempted PLSc bootstrap refits", String(consistentBootstrap.requestedReplicates)],
        ["Usable PLSc bootstrap refits", String(consistentBootstrap.usableReplicates)],
        ["Failed PLSc bootstrap refits", String(consistentBootstrap.failedReplicates)],
        ["Minimum usable PLSc bootstrap refits", String(consistentBootstrap.minimumUsableReplicates)],
        ["Replayable successful PLSc bootstrap witnesses", String(consistentBootstrap.successfulReplicateWitnesses)],
        ["Required PLSc delete-one fits", String(consistentBootstrap.jackknifeCases)],
        ["Replayable successful PLSc delete-one witnesses", String(consistentBootstrap.successfulJackknifeWitnesses)],
        ["Failed PLSc delete-one fits", String(consistentBootstrap.failedJackknifeCases)],
        ["PLSc bootstrap failure policy", "No retry or replacement draw"],
      );
    }
    if (consistentPermutation) {
      const permutation = consistentPermutation.permutation;
      const directional = permutation.directional_inference;
      const selectedTail = consistentPermutation.selectedTailInference;
      rows.push(
        ["PLSc consistent-permutation method", permutation.method_version],
        ["PLSc group estimator", permutation.estimator_method_version!],
        ["PLSc label scheduler", permutation.scheduler_method_version!],
        ["PLSc label operation", permutation.plan.operation],
        ["PLSc permutation test", permutation.test_method!],
        ...(directional ? [
          ["PLSc directional inference method", directional.method_version],
          ["PLSc directional test", directional.test_method],
        ] : []),
        ...(selectedTail ? [
          ["PLSc selected-tail method", selectedTail.method_version],
          ["PLSc selected-tail orientation", "Group A minus Group B"],
          ["PLSc selected test tail", selectedTail.selected_test_tail],
          ["PLSc selected-tail usable denominator", String(consistentPermutation.usablePermutations)],
          ...selectedTail.parameters.flatMap((parameter) => [
            [`PLSc selected exceedances — ${parameter.parameter}`, String(parameter.selected_exceedances)],
            [`PLSc selected p value — ${parameter.parameter}`, String(parameter.selected_p_value)],
            [`PLSc selected usable assignments — ${parameter.parameter}`, String(parameter.permutations)],
          ]),
        ] : []),
        ["PLSc label assignment", "Fixed-size two-group reassignment without replacement with two full PLSc refits per indexed assignment"],
        ["PLSc directed contrast", `${permutation.group_a!.group} minus ${permutation.group_b!.group}`],
        ["PLSc group column", permutation.group_column!],
        ["PLSc Group A complete cases", String(permutation.group_a!.observations)],
        ["PLSc Group B complete cases", String(permutation.group_b!.observations)],
        ["PLSc Group A parameter digest", permutation.group_a!.parameter_values_sha256],
        ["PLSc Group B parameter digest", permutation.group_b!.parameter_values_sha256],
        ["PLSc pooled parameter digest", permutation.pooled_parameter_values_sha256!],
        ["Requested PLSc label assignments", String(consistentPermutation.requestedPermutations)],
        ["Usable PLSc label assignments", String(consistentPermutation.usablePermutations)],
        ["Failed PLSc label assignments", String(consistentPermutation.failedPermutations)],
        ["Minimum usable PLSc label assignments", String(consistentPermutation.minimumUsablePermutations)],
        ["PLSc permutation probability", directional
          ? "Two-tailed absolute and directed greater/less plus-one probabilities share the usable preplanned-assignment denominator"
          : "Two-tailed absolute-difference plus-one probability conditional on usable preplanned assignments"],
        ["PLSc permutation failure policy", "No retry or replacement assignment"],
        ["PLSc permutation limitation", directional
          ? "MICOM, broader parameter/model scope, and more than two groups remain unavailable in this internal v1 result"
          : "MICOM and one-tailed inference are unavailable in this internal v1 result"],
      );
    }
    if (structuralPathRandomization) {
      rows.push(
        ["Randomization method", structuralPathRandomization.methodVersion],
        ["Randomization operation", structuralPathRandomization.operation],
        ["Randomized structural paths", String(structuralPathRandomization.parameters.length)],
        ["Requested path permutations", String(structuralPathRandomization.permutations)],
        ["Randomization estimand", "Structural path coefficients conditional on fixed original PLS construct scores"],
        ["Pathwise probability", "Conditional/approximate two-sided plus-one probability under exchangeable reduced-model residuals; no multiplicity adjustment"],
        ["Availability", "Supported within the documented fixed-score scope"],
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
    if (htmtInference) {
      rows.push(
        ["HTMT bootstrap inference method", htmtInference.method_version],
        ["HTMT+ bootstrap method", htmtInference.htmt_plus.method_version],
        ["Original HTMT bootstrap method", htmtInference.htmt_original.method_version],
        ["HTMT interval", "Bias-corrected percentile (Type 7); not BCa"],
        ["HTMT test", "One-tailed upper, alpha .05"],
        ["HTMT displayed interval", "90% two-sided equivalent (5% and 95% endpoints)"],
        ["HTMT critical value", String(htmtInference.htmt_plus.critical_value)],
        ["HTMT decision rule", "Documented inference: bias-corrected upper bound strictly below 0.90"],
        ["HTMT minimum usable replicates per comparison", String(htmtInference.htmt_plus.minimum_usable_replicates)],
        ["HTMT retry policy", "No retry or replacement of failed preplanned draws"],
        ["HTMT interpretation caution", "The documented 0.90 decision is reported; justify any stricter context-specific criterion separately"],
      );
    }
    if (run.provenance.settings.permutation_samples > 0) {
      rows.push(["Permutation samples", String(run.provenance.settings.permutation_samples)]);
    }
    if (prediction) {
      rows.push(
        ["Prediction targets", "Endogenous indicators; construct scores supplementary"],
        ["Repeated prediction method version", prediction.repeated.method_version],
        ["CVPAT method version", CURRENT_CVPAT_METHOD_VERSION],
        ["Cross-validation design", `${prediction.repeated.folds}-fold × ${prediction.repeated.repeats}-repeat cross-validation`],
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
    if (applicability.usesSeed && run.provenance.method === "mga" && micom?.method_version === "micom_v4") {
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
    status: effectiveStatus,
    warning: structuralPathRandomization
      ? NATIVE_STRUCTURAL_PATH_RANDOMIZATION_WARNING
      : null,
    columns: ["Field", "Value"],
    rows,
  };
}

function currentHtmtBootstrapInference(run: AnalysisRun) {
  const bundle = run.bootstrap?.htmt_inference;
  const marker = run.provenance?.method_version
    .split("+")
    .includes("htmt_bias_corrected_bootstrap_inference_v1") ?? false;
  if (!bundle || !marker
    || bundle.method_version !== "htmt_bias_corrected_bootstrap_inference_v1"
    || bundle.htmt_plus.method_version !== "ringle_et_al_htmt_plus_bias_corrected_bootstrap_v1"
    || bundle.htmt_plus.point_method_version !== "ringle_et_al_htmt_plus_v1"
    || bundle.htmt_plus.absolute_correlations !== true
    || bundle.htmt_original.method_version !== "henseler_et_al_htmt_bias_corrected_bootstrap_v1"
    || bundle.htmt_original.point_method_version !== "henseler_et_al_htmt_v1"
    || bundle.htmt_original.absolute_correlations !== false
    || [bundle.htmt_plus, bundle.htmt_original].some((artifact) => (
      artifact.correlation_type !== "pearson"
      || artifact.interval_method !== "bias_corrected_percentile_type7_v1"
      || artifact.test_type !== "one_tailed_upper"
      || artifact.significance_level !== 0.05
      || artifact.equivalent_two_sided_confidence_level !== 0.9
      || artifact.critical_value !== 0.9
      || artifact.decision_rule !== "bias_corrected_upper_bound_strictly_below_critical_value_v1"
      || artifact.replicate_index_digest_method !== "sha256_u32_le_v1"
      || artifact.retry_policy !== "no_retry_fixed_preplanned_primary_draws_v1"
      || artifact.requested_replicates !== run.bootstrap!.plan.replicates
      || artifact.minimum_usable_replicates !== Math.max(2, Math.ceil(artifact.requested_replicates * 0.9))
    ))) return null;
  return bundle;
}

export function nativePcaScoreExportTable(run: AnalysisRun): ResultTable | null {
  return nativePcaScoreResultTable(run);
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
