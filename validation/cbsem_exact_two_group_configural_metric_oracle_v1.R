#!/usr/bin/env Rscript

# Independent lavaan reference for the quarantined QuickPLS engine prerequisite.
# This script does not qualify or expose a product capability.

args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 2) {
  stop("usage: Rscript cbsem_exact_two_group_configural_metric_oracle_v1.R <fixture.csv> <output.json>")
}
if (!requireNamespace("lavaan", quietly = TRUE)) {
  stop("the lavaan package is required")
}
if (!requireNamespace("jsonlite", quietly = TRUE)) {
  stop("the jsonlite package is required")
}

fixture_path <- args[[1]]
output_path <- args[[2]]
variables <- c("x1", "x2", "x3", "y1", "y2", "y3")
data <- read.csv(fixture_path, stringsAsFactors = FALSE, check.names = FALSE)
if (!identical(names(data), c("group", variables))) {
  stop("fixture columns do not match the frozen reference contract")
}
if (!identical(sort(unique(data$group)), c("A", "B"))) {
  stop("fixture must contain exactly groups A and B")
}
if (any(!complete.cases(data[, c("group", variables)]))) {
  stop("fixture contains missing values")
}
data$group <- factor(data$group, levels = c("A", "B"))

model_syntax <- paste(
  "f1 =~ x1 + x2 + x3",
  "f2 =~ y1 + y2 + y3",
  "f1 ~~ f2",
  sep = "\n"
)
common_args <- list(
  model = model_syntax,
  data = data,
  group = "group",
  estimator = "ML",
  missing = "listwise",
  meanstructure = FALSE,
  fixed.x = FALSE,
  std.lv = FALSE,
  auto.fix.first = TRUE
)
configural <- do.call(lavaan::cfa, common_args)
metric_args <- common_args
metric_args$group.equal <- "loadings"
metric <- do.call(lavaan::cfa, metric_args)

if (!lavaan::lavInspect(configural, "converged") || !lavaan::lavInspect(metric, "converged")) {
  stop("lavaan reference fit did not converge")
}

cov_ml <- function(frame) {
  matrix <- as.matrix(frame[, variables, drop = FALSE])
  centered <- sweep(matrix, 2, colMeans(matrix), FUN = "-")
  crossprod(centered) / nrow(matrix)
}

fml <- function(sample_covariance, implied_covariance) {
  sample_logdet <- as.numeric(determinant(sample_covariance, logarithm = TRUE)$modulus)
  implied_logdet <- as.numeric(determinant(implied_covariance, logarithm = TRUE)$modulus)
  implied_logdet + sum(diag(solve(implied_covariance, sample_covariance))) -
    sample_logdet - nrow(sample_covariance)
}

parameter_rows <- function(fit) {
  table <- lavaan::parTable(fit)
  keep <- table$op %in% c("=~", "~~") &
    ((table$op == "=~") |
      (table$lhs %in% c("f1", "f2") & table$rhs %in% c("f1", "f2")) |
      (table$lhs %in% variables & table$lhs == table$rhs))
  table <- table[keep, c("lhs", "op", "rhs", "group", "free", "ustart", "est", "se")]
  lapply(seq_len(nrow(table)), function(index) {
    row <- table[index, ]
    list(
      lhs = as.character(row$lhs),
      op = as.character(row$op),
      rhs = as.character(row$rhs),
      group = as.integer(row$group),
      estimate = as.numeric(row$est),
      standard_error = if (is.finite(row$se)) as.numeric(row$se) else NULL,
      fixed = as.integer(row$free) == 0L,
      fixed_start = if (is.finite(row$ustart)) as.numeric(row$ustart) else NULL
    )
  })
}

fit_projection <- function(fit, model_name) {
  implied <- lavaan::lavInspect(fit, "implied")
  split_rows <- split(data, data$group, drop = TRUE)
  group_rows <- lapply(seq_along(split_rows), function(index) {
    frame <- split_rows[[index]]
    sample_covariance <- cov_ml(frame)
    implied_covariance <- implied[[index]]$cov
    objective <- fml(sample_covariance, implied_covariance)
    list(
      group = names(split_rows)[[index]],
      sample_size = nrow(frame),
      observed_means = unname(as.numeric(colMeans(frame[, variables, drop = FALSE]))),
      covariance_ml = unname(split(sample_covariance, row(sample_covariance))),
      implied_covariance = unname(split(implied_covariance, row(implied_covariance))),
      objective = objective,
      chi_square = nrow(frame) * objective
    )
  })
  measures <- lavaan::fitMeasures(fit, c("chisq", "df"))
  list(
    model = model_name,
    converged = TRUE,
    free_dimensions = as.integer(lavaan::lavInspect(fit, "npar")),
    degrees_of_freedom = as.integer(measures[["df"]]),
    chi_square = as.numeric(measures[["chisq"]]),
    group_fits = group_rows,
    parameters = parameter_rows(fit)
  )
}

configural_projection <- fit_projection(configural, "configural")
metric_projection <- fit_projection(metric, "metric")
delta_chi_square <- metric_projection$chi_square - configural_projection$chi_square
delta_df <- metric_projection$degrees_of_freedom - configural_projection$degrees_of_freedom

selected_options <- function(fit) {
  options <- lavaan::lavInspect(fit, "options")
  list(
    estimator = as.character(options$estimator),
    likelihood = as.character(options$likelihood),
    missing = as.character(options$missing),
    meanstructure = isTRUE(options$meanstructure),
    fixed_x = isTRUE(options$fixed.x),
    std_lv = isTRUE(options$std.lv),
    auto_fix_first = isTRUE(options$auto.fix.first),
    sample_cov_rescale = isTRUE(options$sample.cov.rescale)
  )
}

output <- list(
  schema_version = 1L,
  kind = "cbsem_exact_two_group_configural_metric_lavaan_reference_v1",
  status = "independent_external_engine_reference_not_product_qualification",
  runtime = list(
    r_version = R.version.string,
    lavaan_version = as.character(utils::packageVersion("lavaan")),
    jsonlite_version = as.character(utils::packageVersion("jsonlite"))
  ),
  contract = list(
    estimator = "ML",
    missing = "listwise",
    meanstructure = FALSE,
    fixed_x = FALSE,
    marker_identification = TRUE,
    configural_group_equal = list(),
    metric_group_equal = list("loadings"),
    observed_variable_order = variables,
    model_syntax = model_syntax
  ),
  observed_options = list(
    configural = selected_options(configural),
    metric = selected_options(metric)
  ),
  group_sample_sizes = lapply(split(data, data$group), nrow),
  configural = configural_projection,
  metric = metric_projection,
  likelihood_ratio_test = list(
    statistic = delta_chi_square,
    degrees_of_freedom = delta_df,
    upper_tail_p_value = stats::pchisq(delta_chi_square, df = delta_df, lower.tail = FALSE)
  )
)

jsonlite::write_json(output, output_path, auto_unbox = TRUE, pretty = TRUE, digits = 17, null = "null")
