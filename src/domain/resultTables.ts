import type { AnalysisRun, PlsResult } from "../types";

export interface ResultTable {
  id: string;
  title: string;
  status: "validated" | "experimental";
  warning: string | null;
  columns: string[];
  rows: string[][];
}

const SCOPE_WARNING = "Validated for the documented QuickPLS v0.9.0-rc.1 supported scope. Unsupported shapes remain blocked or explicitly marked.";

export function methodResultTables(result: PlsResult): ResultTable[] {
  const tables: ResultTable[] = [];
  if (result.plsc) {
    tables.push({
      id: "plsc_reliability",
      title: "PLSc reliabilities",
      status: "experimental",
      warning: warnings(result.plsc.warnings),
      columns: ["Construct", "rho_A"],
      rows: result.plsc.reliabilities.map((row) => [row.construct, formatNumber(row.rho_a, 6)]),
    });
    tables.push({
      id: "plsc_correlations",
      title: "PLSc corrected construct correlations",
      status: "experimental",
      warning: warnings(result.plsc.warnings),
      columns: ["Left", "Right", "Original", "Corrected"],
      rows: result.plsc.construct_correlations.map((row) => [row.left, row.right, formatNumber(row.original, 6), formatNumber(row.corrected, 6)]),
    });
    tables.push({
      id: "plsc_paths",
      title: "PLSc corrected paths",
      status: "experimental",
      warning: warnings(result.plsc.warnings),
      columns: ["Source", "Target", "Coefficient"],
      rows: result.plsc.corrected_paths.map((row) => [row.source, row.target, formatNumber(row.coefficient, 6)]),
    });
  }

  if (result.wpls) {
    tables.push({
      id: "wpls_weights",
      title: "WPLS case-weight metadata",
      status: "experimental",
      warning: warnings(result.wpls.warnings),
      columns: ["Weight column", "Weight sum", "Effective sample size", "Covariance"],
      rows: [[result.wpls.case_weight_column, formatNumber(result.wpls.weight_sum, 6), formatNumber(result.wpls.effective_sample_size, 4), formatLabel(result.wpls.covariance)]],
    });
  }

  if (result.cca) {
    tables.push({
      id: "cca_residuals",
      title: "CCA composite residuals",
      status: "experimental",
      warning: warnings(result.cca.warnings),
      columns: ["Left", "Right", "Observed", "Reproduced", "Residual", "Absolute residual"],
      rows: result.cca.correlations.map((row) => [row.left, row.right, formatNumber(row.observed, 6), formatNumber(row.reproduced, 6), formatNumber(row.residual, 6), formatNumber(row.absolute_residual, 6)]),
    });
    tables.push({
      id: "cca_summary",
      title: "CCA residual summary",
      status: "experimental",
      warning: warnings(result.cca.warnings),
      columns: ["Metric", "Value"],
      rows: [["Max absolute residual", formatNumber(result.cca.max_absolute_residual, 6)]],
    });
  }

  if (result.predict) {
    tables.push({
      id: "plspredict_holdout",
      title: "PLSpredict holdout metrics",
      status: "experimental",
      warning: warnings(result.predict.warnings),
      columns: ["Construct", "Predictors", "RMSE PLS", "MAE PLS", "RMSE benchmark", "MAE benchmark", "Q2 predict", "RMSE LM", "MAE LM", "Q2 LM"],
      rows: result.predict.targets.map((row) => [
        row.construct,
        String(row.predictor_count),
        formatNumber(row.rmse_pls, 6),
        formatNumber(row.mae_pls, 6),
        formatNumber(row.rmse_benchmark, 6),
        formatNumber(row.mae_benchmark, 6),
        row.q_squared_predict == null ? "N/A" : formatNumber(row.q_squared_predict, 6),
        row.rmse_lm == null ? "N/A" : formatNumber(row.rmse_lm, 6),
        row.mae_lm == null ? "N/A" : formatNumber(row.mae_lm, 6),
        row.q_squared_predict_lm == null ? "N/A" : formatNumber(row.q_squared_predict_lm, 6),
      ]),
    });
    tables.push({
      id: "plspredict_split",
      title: "PLSpredict holdout split",
      status: "experimental",
      warning: warnings(result.predict.warnings),
      columns: ["Split", "Training observations", "Test observations", "Benchmark"],
      rows: [[formatLabel(result.predict.split), String(result.predict.training_observations), String(result.predict.test_observations), result.predict.benchmark]],
    });
    if (result.predict.repeated_kfold) {
      tables.push({
        id: "plspredict_repeated_kfold",
        title: "PLSpredict repeated k-fold metrics",
        status: "experimental",
        warning: warnings(result.predict.repeated_kfold.warnings),
        columns: ["Construct", "Predictors", "RMSE PLS", "MAE PLS", "RMSE benchmark", "MAE benchmark", "Q2 predict", "RMSE LM", "MAE LM", "Q2 LM"],
        rows: result.predict.repeated_kfold.targets.map((row) => [
          row.construct,
          String(row.predictor_count),
          formatNumber(row.rmse_pls, 6),
          formatNumber(row.mae_pls, 6),
          formatNumber(row.rmse_benchmark, 6),
          formatNumber(row.mae_benchmark, 6),
          row.q_squared_predict == null ? "N/A" : formatNumber(row.q_squared_predict, 6),
          row.rmse_lm == null ? "N/A" : formatNumber(row.rmse_lm, 6),
          row.mae_lm == null ? "N/A" : formatNumber(row.mae_lm, 6),
          row.q_squared_predict_lm == null ? "N/A" : formatNumber(row.q_squared_predict_lm, 6),
        ]),
      });
      tables.push({
        id: "plspredict_repeated_kfold_plan",
        title: "PLSpredict repeated k-fold plan",
        status: "experimental",
        warning: warnings(result.predict.repeated_kfold.warnings),
        columns: ["Folds", "Repeats", "Total test observations", "Assignment"],
        rows: [[String(result.predict.repeated_kfold.folds), String(result.predict.repeated_kfold.repeats), String(result.predict.repeated_kfold.total_test_observations), formatLabel(result.predict.repeated_kfold.assignment)]],
      });
      if (result.predict.repeated_kfold.cvpat?.length) {
        tables.push({
          id: "cvpat",
          title: "CVPAT paired loss comparisons",
          status: "experimental",
          warning: warnings(result.predict.repeated_kfold.warnings),
          columns: ["Target", "Comparison", "Mean loss diff", "SE", "t", "p", "Observations", "Preferred", "Warning"],
          rows: result.predict.repeated_kfold.cvpat.map((row) => [
            row.target,
            formatLabel(row.comparison),
            formatNumber(row.mean_loss_difference, 6),
            row.standard_error == null ? "N/A" : formatNumber(row.standard_error, 6),
            row.t_statistic == null ? "N/A" : formatNumber(row.t_statistic, 4),
            formatPValue(row.p_value_two_sided),
            String(row.observations),
            formatLabel(row.preferred_model),
            row.warning ?? "",
          ]),
        });
      }
    }
  }

  if (result.cta_pls) {
    tables.push({
      id: "cta_pls_summary",
      title: "CTA-PLS tetrad summary",
      status: "experimental",
      warning: warnings(result.cta_pls.warnings),
      columns: ["Construct", "Max absolute tetrad"],
      rows: Object.entries(result.cta_pls.max_absolute_tetrad_by_construct).map(([construct, value]) => [construct, formatNumber(value, 6)]),
    });
    tables.push({
      id: "cta_pls_tetrads",
      title: "CTA-PLS tetrads",
      status: "experimental",
      warning: warnings(result.cta_pls.warnings),
      columns: ["Construct", "Indicator A", "Indicator B", "Indicator C", "Indicator D", "Pairing", "Tetrad", "Absolute tetrad"],
      rows: result.cta_pls.estimates.map((row) => [row.construct, row.indicator_a, row.indicator_b, row.indicator_c, row.indicator_d, formatLabel(row.pairing), formatNumber(row.tetrad, 6), formatNumber(row.absolute_tetrad, 6)]),
    });
  }

  if (result.pca) {
    tables.push({
      id: "pca_components",
      title: "PCA components",
      status: "experimental",
      warning: warnings(result.pca.warnings),
      columns: ["Component", "Eigenvalue", "Explained variance", "Cumulative variance"],
      rows: result.pca.components.map((row) => [row.component, formatNumber(row.eigenvalue, 6), formatNumber(row.explained_variance, 6), formatNumber(row.cumulative_variance, 6)]),
    });
    tables.push({
      id: "pca_loadings",
      title: "PCA loadings",
      status: "experimental",
      warning: warnings(result.pca.warnings),
      columns: ["Variable", "Component", "Loading", "Weight"],
      rows: result.pca.loadings.map((row) => [row.variable, row.component, formatNumber(row.loading, 6), formatNumber(row.weight, 6)]),
    });
  }

  if (result.regression) {
    tables.push({
      id: "regression_coefficients",
      title: "Regression coefficients",
      status: "experimental",
      warning: warnings(result.regression.warnings),
      columns: ["Term", "Estimate", "SE", "Statistic", "p", "Lower", "Upper", "Odds ratio"],
      rows: result.regression.coefficients.map((row) => [row.term, formatNumber(row.estimate, 6), formatNumber(row.standard_error, 6), formatNumber(row.statistic, 4), formatPValue(row.p_value_two_sided), formatNumber(row.confidence_interval_lower, 6), formatNumber(row.confidence_interval_upper, 6), row.odds_ratio == null ? "N/A" : formatNumber(row.odds_ratio, 6)]),
    });
    tables.push({
      id: "regression_fit",
      title: "Regression fit",
      status: "experimental",
      warning: warnings(result.regression.warnings),
      columns: ["Metric", "Value"],
      rows: [
        ["Type", formatLabel(result.regression.regression_type)],
        ["Outcome", result.regression.outcome],
        ["Observations", String(result.regression.observations)],
        ["R2", result.regression.fit.r_squared == null ? "N/A" : formatNumber(result.regression.fit.r_squared, 6)],
        ["Adjusted R2", result.regression.fit.adjusted_r_squared == null ? "N/A" : formatNumber(result.regression.fit.adjusted_r_squared, 6)],
        ["Pseudo R2", result.regression.fit.pseudo_r_squared == null ? "N/A" : formatNumber(result.regression.fit.pseudo_r_squared, 6)],
        ["AIC", formatNumber(result.regression.fit.aic, 6)],
        ["BIC", formatNumber(result.regression.fit.bic, 6)],
      ],
    });
    if (result.regression.process) {
      tables.push({
        id: "process_effects",
        title: "PROCESS-style effects",
        status: "experimental",
        warning: warnings(result.regression.process.warnings),
        columns: ["Effect", "Estimate", "Lower", "Upper"],
        rows: result.regression.process.effects.map((row) => [formatLabel(row.effect), formatNumber(row.estimate, 6), row.lower_percentile == null ? "N/A" : formatNumber(row.lower_percentile, 6), row.upper_percentile == null ? "N/A" : formatNumber(row.upper_percentile, 6)]),
      });
    }
  }

  if (result.nca) {
    tables.push({
      id: "nca_ceilings",
      title: "NCA ceiling effects",
      status: "experimental",
      warning: warnings(result.nca.warnings),
      columns: ["Ceiling", "Effect size", "Permutation p"],
      rows: result.nca.ceilings.map((row) => [formatLabel(row.ceiling), formatNumber(row.effect_size, 6), formatPValue(row.permutation_p_value)]),
    });
    tables.push({
      id: "nca_bottlenecks",
      title: "NCA bottleneck table",
      status: "experimental",
      warning: warnings(result.nca.warnings),
      columns: ["Outcome %", "Required X %"],
      rows: result.nca.bottlenecks.map((row) => [formatNumber(row.outcome_percent, 1), formatNumber(row.required_x_percent, 4)]),
    });
  }

  if (result.gsca) {
    tables.push({
      id: "gsca_fit",
      title: "GSCA fit diagnostics",
      status: "experimental",
      warning: warnings(result.gsca.warnings),
      columns: ["Metric", "Value"],
      rows: [["FIT", formatNumber(result.gsca.fit, 6)], ["AFIT", formatNumber(result.gsca.adjusted_fit, 6)], ["GFI", formatNumber(result.gsca.gfi, 6)], ["Iterations", String(result.gsca.iterations)]],
    });
    tables.push({
      id: "gsca_paths",
      title: "GSCA paths",
      status: "experimental",
      warning: warnings(result.gsca.warnings),
      columns: ["Source", "Target", "Coefficient"],
      rows: result.gsca.paths.map((row) => [row.source, row.target, formatNumber(row.coefficient, 6)]),
    });
  }

  if (result.segmentation) {
    tables.push({
      id: "segmentation_summary",
      title: "PLS-POS bounded segmentation summary",
      status: "experimental",
      warning: warnings(result.segmentation.warnings),
      columns: ["Algorithm", "Requested", "Selected", "Observations", "Objective", "Pooled objective", "Improvement", "Min share", "Imbalance", "Max path separation", "Assignment"],
      rows: [[
        formatLabel(result.segmentation.algorithm),
        String(result.segmentation.requested_segments),
        String(result.segmentation.selected_segments),
        String(result.segmentation.observations),
        formatNumber(result.segmentation.objective, 6),
        formatNumber(result.segmentation.pooled_objective, 6),
        formatNumber(result.segmentation.objective_improvement, 6),
        formatNumber(result.segmentation.min_segment_share, 4),
        formatNumber(result.segmentation.segment_size_imbalance, 4),
        formatNumber(result.segmentation.max_path_separation, 6),
        result.segmentation.assignment,
      ]],
    });
    tables.push({
      id: "segmentation_segments",
      title: "PLS-POS bounded segment paths",
      status: "experimental",
      warning: warnings(result.segmentation.warnings),
      columns: ["Segment", "Observations", "Share", "Source", "Target", "Path coefficient", "R2"],
      rows: result.segmentation.segments.flatMap((segment) => segment.paths.map((path) => [
        formatLabel(segment.segment),
        String(segment.observations),
        formatNumber(segment.share, 4),
        path.source,
        path.target,
        formatNumber(path.coefficient, 6),
        formatNumber(segment.r_squared[path.target] ?? Number.NaN, 6),
      ])),
    });
    if (result.segmentation.memberships?.length) {
      tables.push({
        id: "segmentation_memberships",
        title: "PLS-POS bounded segment memberships",
        status: "experimental",
        warning: warnings(result.segmentation.warnings),
        columns: ["Observation", "Segment"],
        rows: result.segmentation.memberships.map((membership) => [
          String(membership.observation),
          formatLabel(membership.segment),
        ]),
      });
    }
  }

  if (result.mga) {
    tables.push({
      id: "mga_summary",
      title: "MGA two-group summary",
      status: "experimental",
      warning: warnings(result.mga.warnings),
      columns: ["Group column", "Groups", "Comparisons", "Method version"],
      rows: [[
        result.mga.group_column,
        result.mga.groups.map((group) => `${group.group} (${group.observations})`).join("; "),
        String(result.mga.comparisons.length),
        result.mga.method_version,
      ]],
    });
    tables.push({
      id: "mga_paths",
      title: "MGA group paths",
      status: "experimental",
      warning: warnings(result.mga.warnings),
      columns: ["Group", "Observations", "Source", "Target", "Path coefficient", "R2"],
      rows: result.mga.groups.flatMap((group) => group.paths.map((path) => [
        group.group,
        String(group.observations),
        path.source,
        path.target,
        formatNumber(path.coefficient, 6),
        formatNumber(group.r_squared[path.target] ?? Number.NaN, 6),
      ])),
    });
    tables.push({
      id: "mga_comparisons",
      title: "MGA path comparisons",
      status: "experimental",
      warning: warnings(result.mga.warnings),
      columns: ["Source", "Target", "Group A", "Coefficient A", "Group B", "Coefficient B", "Difference", "SE", "t", "p", "Warning"],
      rows: result.mga.comparisons.map((comparison) => [
        comparison.source,
        comparison.target,
        comparison.group_a,
        formatNumber(comparison.coefficient_a, 6),
        comparison.group_b,
        formatNumber(comparison.coefficient_b, 6),
        formatNumber(comparison.difference, 6),
        comparison.standard_error == null ? "N/A" : formatNumber(comparison.standard_error, 6),
        comparison.t_statistic == null ? "N/A" : formatNumber(comparison.t_statistic, 4),
        formatPValue(comparison.p_value_two_sided),
        comparison.warning ?? "",
      ]),
    });
  }

  if (result.micom) {
    tables.push({
      id: "micom_constructs",
      title: "MICOM measurement invariance",
      status: "experimental",
      warning: warnings(result.micom.warnings),
      columns: ["Construct", "Configural", "Composition corr", "Composition p", "Mean diff", "Mean p", "Variance diff", "Variance p", "Partial", "Full"],
      rows: result.micom.constructs.map((row) => [
        row.construct,
        row.configural_invariance ? "yes" : "no",
        formatNumber(row.compositional_correlation, 6),
        formatPValue(row.compositional_p_value),
        formatNumber(row.mean_difference, 6),
        formatPValue(row.mean_p_value),
        formatNumber(row.variance_difference, 6),
        formatPValue(row.variance_p_value),
        row.partial_invariance ? "yes" : "no",
        row.full_invariance ? "yes" : "no",
      ]),
    });
  }

  if (result.mga_permutation) {
    tables.push({
      id: "mga_permutation",
      title: "Permutation MGA path differences",
      status: "experimental",
      warning: warnings(result.mga_permutation.warnings),
      columns: ["Source", "Target", "Original difference", "Empirical p", "Percentile rank"],
      rows: result.mga_permutation.comparisons.map((row) => [
        row.source,
        row.target,
        formatNumber(row.original_difference, 6),
        formatPValue(row.empirical_p_value_two_sided),
        row.percentile_rank == null ? "N/A" : formatNumber(row.percentile_rank, 4),
      ]),
    });
  }

  if (result.fimix) {
    tables.push({
      id: "fimix_summary",
      title: "FIMIX-PLS class summary",
      status: "experimental",
      warning: warnings(result.fimix.warnings),
      columns: ["Classes", "Starts", "Iterations", "Log likelihood", "AIC", "BIC", "CAIC", "Entropy"],
      rows: [[
        String(result.fimix.classes),
        String(result.fimix.starts),
        String(result.fimix.iterations),
        formatNumber(result.fimix.log_likelihood, 6),
        formatNumber(result.fimix.aic, 6),
        formatNumber(result.fimix.bic, 6),
        formatNumber(result.fimix.caic, 6),
        formatNumber(result.fimix.entropy, 4),
      ]],
    });
    tables.push({
      id: "fimix_paths",
      title: "FIMIX-PLS class paths",
      status: "experimental",
      warning: warnings(result.fimix.warnings),
      columns: ["Class", "Observations", "Share", "Source", "Target", "Path coefficient", "R2"],
      rows: result.fimix.classes_summary.flatMap((item) => item.paths.map((path) => [
        item.class,
        String(item.observations),
        formatNumber(item.share, 4),
        path.source,
        path.target,
        formatNumber(path.coefficient, 6),
        formatNumber(item.r_squared[path.target] ?? Number.NaN, 6),
      ])),
    });
  }

  if (result.ipma) {
    tables.push({
      id: "ipma_constructs",
      title: "IPMA construct importance-performance",
      status: "experimental",
      warning: warnings(result.ipma.warnings),
      columns: ["Target", "Construct", "Importance", "Performance", "Score mean"],
      rows: result.ipma.constructs.map((row) => [
        row.target,
        row.construct,
        formatNumber(row.importance, 6),
        formatNumber(row.performance, 4),
        formatNumber(row.score_mean, 6),
      ]),
    });
    tables.push({
      id: "ipma_indicators",
      title: "IPMA indicator performance",
      status: "experimental",
      warning: warnings(result.ipma.warnings),
      columns: ["Target", "Construct", "Indicator", "Construct importance", "Loading", "Performance", "Score mean"],
      rows: result.ipma.indicators.map((row) => [
        row.target,
        row.construct,
        row.indicator,
        formatNumber(row.construct_importance, 6),
        formatNumber(row.loading, 6),
        formatNumber(row.performance, 4),
        formatNumber(row.score_mean, 6),
      ]),
    });
  }

  if (result.cbsem) {
    tables.push({
      id: "cbsem_fit",
      title: "CB-SEM fit indices",
      status: "experimental",
      warning: warnings(result.cbsem.warnings.concat(result.cbsem.diagnostics)),
      columns: ["Metric", "Value"],
      rows: [
        ["Method version", result.cbsem.method_version],
        ["Model type", result.cbsem.model_type],
        ["Estimator", result.cbsem.estimator],
        ["Input", result.cbsem.input],
        ["Sample size", String(result.cbsem.sample_size)],
        ["chi-square", formatNumber(result.cbsem.fit.chi_square, 6)],
        ["df", String(result.cbsem.fit.degrees_of_freedom)],
        ["p value", formatPValue(result.cbsem.fit.p_value)],
        ["CFI", result.cbsem.fit.cfi == null ? "N/A" : formatNumber(result.cbsem.fit.cfi, 6)],
        ["TLI", result.cbsem.fit.tli == null ? "N/A" : formatNumber(result.cbsem.fit.tli, 6)],
        ["RMSEA", result.cbsem.fit.rmsea == null ? "N/A" : formatNumber(result.cbsem.fit.rmsea, 6)],
        ["SRMR", formatNumber(result.cbsem.fit.srmr, 6)],
        ["AIC", formatNumber(result.cbsem.fit.aic, 6)],
        ["BIC", formatNumber(result.cbsem.fit.bic, 6)],
      ],
    });
    tables.push({
      id: "cbsem_parameters",
      title: "CB-SEM parameter estimates",
      status: "experimental",
      warning: warnings(result.cbsem.warnings),
      columns: ["Parameter", "Kind", "LHS", "RHS", "Estimate", "SE", "z", "p", "Fixed", "Warning"],
      rows: result.cbsem.parameters.map((row) => [row.name, formatLabel(row.kind), row.lhs, row.rhs, formatNumber(row.estimate, 6), row.standard_error == null ? "N/A" : formatNumber(row.standard_error, 6), row.z_statistic == null ? "N/A" : formatNumber(row.z_statistic, 4), formatPValue(row.p_value_two_sided), row.fixed ? "yes" : "no", row.warning ?? ""]),
    });
    tables.push({
      id: "cbsem_standardized",
      title: "CB-SEM standardized solution",
      status: "experimental",
      warning: warnings(result.cbsem.warnings),
      columns: ["Parameter", "Kind", "LHS", "RHS", "std_lv", "std_all"],
      rows: result.cbsem.standardized.map((row) => [row.name, formatLabel(row.kind), row.lhs, row.rhs, formatNumber(row.std_lv, 6), formatNumber(row.std_all, 6)]),
    });
    tables.push({
      id: "cbsem_modification_indices",
      title: "CB-SEM modification indices",
      status: "experimental",
      warning: warnings(result.cbsem.warnings),
      columns: ["Kind", "LHS", "RHS", "MI", "EPC"],
      rows: result.cbsem.modification_indices.map((row) => [formatLabel(row.kind), row.lhs, row.rhs, formatNumber(row.modification_index, 6), row.expected_parameter_change == null ? "N/A" : formatNumber(row.expected_parameter_change, 6)]),
    });
    if (result.cbsem.bootstrap) {
      tables.push({
        id: "cbsem_bootstrap",
        title: "CB-SEM bootstrap intervals",
        status: "experimental",
        warning: warnings(result.cbsem.bootstrap.warnings),
        columns: ["Parameter", "Original", "Lower percentile", "Upper percentile"],
        rows: result.cbsem.bootstrap.intervals.map((row) => [row.parameter, formatNumber(row.original, 6), formatNumber(row.lower_percentile, 6), formatNumber(row.upper_percentile, 6)]),
      });
    }
    if (result.cbsem.multigroup) {
      tables.push({
        id: "cbsem_multigroup",
        title: "CB-SEM multigroup fit",
        status: "experimental",
        warning: warnings(result.cbsem.multigroup.warnings),
        columns: ["Group", "Observations", "chi-square", "df", "CFI", "RMSEA"],
        rows: result.cbsem.multigroup.groups.map((row) => [row.group, String(row.observations), formatNumber(row.chi_square, 6), String(row.degrees_of_freedom), row.cfi == null ? "N/A" : formatNumber(row.cfi, 6), row.rmsea == null ? "N/A" : formatNumber(row.rmsea, 6)]),
      });
      tables.push({
        id: "cbsem_invariance",
        title: "CB-SEM invariance steps",
        status: "experimental",
        warning: warnings(result.cbsem.multigroup.warnings),
        columns: ["Step", "chi-square", "df", "Delta chi-square", "Delta df", "Delta CFI", "Delta RMSEA", "Warning"],
        rows: result.cbsem.multigroup.invariance.map((row) => [formatLabel(row.step), formatNumber(row.chi_square, 6), String(row.degrees_of_freedom), row.delta_chi_square == null ? "N/A" : formatNumber(row.delta_chi_square, 6), row.delta_degrees_of_freedom == null ? "N/A" : String(row.delta_degrees_of_freedom), row.delta_cfi == null ? "N/A" : formatNumber(row.delta_cfi, 6), row.delta_rmsea == null ? "N/A" : formatNumber(row.delta_rmsea, 6), row.warning ?? ""]),
      });
    }
  }

  if (result.endogeneity) {
    tables.push({
      id: "endogeneity_copula",
      title: "Gaussian-copula endogeneity diagnostics",
      status: "experimental",
      warning: warnings(result.endogeneity.warnings),
      columns: ["Source", "Target", "Path coefficient", "Copula coefficient", "t statistic", "p value", "Predictor skewness", "Applicability", "Warning"],
      rows: result.endogeneity.estimates.map((row) => [row.source, row.target, formatNumber(row.path_coefficient, 6), formatNumber(row.copula_coefficient, 6), formatNumber(row.t_statistic, 4), formatPValue(row.p_value_two_sided), formatNumber(row.predictor_skewness, 4), row.applicable ? "screenable" : "weak", row.warning ?? ""]),
    });
  }

  if (result.nonlinear_effects) {
    tables.push({
      id: "nonlinear_effects",
      title: "Nonlinear effects",
      status: "experimental",
      warning: warnings(result.nonlinear_effects.warnings),
      columns: ["Source", "Target", "Linear coefficient", "Quadratic coefficient", "t statistic", "p value", "Linear R2", "Augmented R2", "Delta R2", "Warning"],
      rows: result.nonlinear_effects.estimates.map((row) => [row.source, row.target, formatNumber(row.linear_coefficient, 6), formatNumber(row.quadratic_coefficient, 6), formatNumber(row.t_statistic, 4), formatPValue(row.p_value_two_sided), formatNumber(row.linear_r_squared, 4), formatNumber(row.augmented_r_squared, 4), formatNumber(row.delta_r_squared, 4), row.warning ?? ""]),
    });
  }

  if (result.moderated_mediation) {
    tables.push({
      id: "moderated_mediation",
      title: "Moderated mediation",
      status: "experimental",
      warning: warnings(result.moderated_mediation.warnings),
      columns: ["Interaction", "Predictor", "Moderator", "Mediator", "Target", "Stage", "Index", "Conditional indirect effects", "Warning"],
      rows: result.moderated_mediation.estimates.map((row) => [
        row.interaction,
        row.predictor,
        row.moderator,
        row.mediator,
        row.target,
        formatLabel(row.moderated_stage),
        formatNumber(row.index_of_moderated_mediation, 6),
        row.conditional_indirect_effects.map((effect) => `${formatModeratorLevel(effect.moderator_score)}=${formatNumber(effect.indirect_effect, 6)}`).join("; "),
        row.warning ?? "",
      ]),
    });
  }

  return tables;
}

export function runExportTables(run: AnalysisRun): ResultTable[] {
  if (!run.result) return [];
  return [
    {
      id: "run_provenance",
      title: "Run provenance",
      status: run.result.method_version === "pls_pm_v1" ? "validated" : "experimental",
      warning: run.result.method_version === "pls_pm_v1" ? null : SCOPE_WARNING,
      columns: ["Field", "Value"],
      rows: [
        ["Run", run.name],
        ["Method", run.method],
        ["Created at", run.createdAt],
        ["Seed", String(run.seed)],
        ["Dataset fingerprint", run.fingerprint],
        ["Method version", run.result.method_version],
      ],
    },
    ...methodResultTables(run.result),
  ];
}

export function tablesToCsv(tables: ResultTable[]): string {
  return tables.flatMap((table) => [
    [table.title],
    ["Status", table.status],
    ["Warning", table.warning ?? ""],
    table.columns,
    ...table.rows,
    [],
  ]).map((row) => row.map(csvCell).join(",")).join("\r\n");
}

export function tablesToHtml(tables: ResultTable[]): string {
  const sections = tables.map((table) => `<section>
<h2>${escapeHtml(table.title)}</h2>
<p><strong>Status:</strong> ${escapeHtml(table.status)}</p>
${table.warning ? `<p><strong>Warning:</strong> ${escapeHtml(table.warning)}</p>` : ""}
<table><thead><tr>${table.columns.map((column) => `<th>${escapeHtml(column)}</th>`).join("")}</tr></thead><tbody>
${table.rows.map((row) => `<tr>${row.map((cell) => `<td>${escapeHtml(cell)}</td>`).join("")}</tr>`).join("\n")}
</tbody></table>
</section>`).join("\n");
  return `<!doctype html><html><head><meta charset="utf-8"><title>QuickPLS export</title><style>body{font-family:Arial,sans-serif;margin:24px;color:#172126}table{border-collapse:collapse;margin:8px 0 20px;width:100%}th,td{border:1px solid #ccd5d9;padding:6px 8px;text-align:left}th{background:#eef3f4}p{color:#4d5b62}</style></head><body><h1>QuickPLS result export</h1>${sections}</body></html>`;
}

function warnings(values: string[]) {
  return values.length ? `${SCOPE_WARNING} ${values.join(" ")}` : SCOPE_WARNING;
}

function formatNumber(value: number, digits: number) {
  return Number.isFinite(value) ? value.toFixed(digits) : "N/A";
}

function formatPValue(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return "N/A";
  return value < 0.0001 ? "<0.0001" : value.toFixed(4);
}

function formatLabel(code: string) {
  return code.replace(/^(rho_a|htmt)\./, "").replaceAll("_", " ");
}

function formatModeratorLevel(value: number) {
  if (value === -1) return "-1 SD";
  if (value === 0) return "Mean";
  if (value === 1) return "+1 SD";
  return value.toFixed(2);
}

function csvCell(value: string) {
  return /[",\r\n]/.test(value) ? `"${value.replaceAll("\"", "\"\"")}"` : value;
}

function escapeHtml(value: string) {
  return value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;").replaceAll("\"", "&quot;");
}
