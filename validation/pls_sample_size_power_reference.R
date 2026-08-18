#!/usr/bin/env Rscript
# Independent base-R reference for the bounded pls_sample_size_power_v1 method.
# It intentionally does not load or call QuickPLS. Production recipe I/O uses
# jsonlite when available; all analytical equations below use base R only.

METHOD_VERSION <- "pls_sample_size_power_v1"
CAPABILITY_ID <- "qpls3.pls.sample_size_power"
STREAM_DOMAIN <- "quickpls/pls_sample_size_power_v1/independent_r"
FAILURE_POLICY <- "failed_replicates_count_as_non_rejections_v1"
INTERVAL_METHOD <- "wilson_score_two_sided_v1"
INFERENCE_METHOD <- "pls_pm_case_bootstrap_normal_reference_two_sided_v1"
MAX_ESTIMATED_PLS_FITS <- 250000
MAX_ESTIMATED_PLS_CASE_FITS <- 100000000

stop_field <- function(field, message) {
  stop(sprintf("%s: %s", field, message), call. = FALSE)
}

validate_recipe <- function(recipe, production_counts = TRUE) {
  expected <- sort(c(
    "schema_version", "capability_id", "method_version", "scenario_identity",
    "design", "estimator", "inference", "sample_size_grid", "alpha",
    "target_power", "confidence_level", "monte_carlo_replicates",
    "bootstrap_replicates", "master_seed", "workers"
  ))
  if (!identical(sort(names(recipe)), expected)) stop_field("recipe", "unknown or missing fields")
  if (!identical(as.integer(recipe$schema_version), 1L)) stop_field("schema_version", "must equal 1")
  if (!identical(recipe$capability_id, CAPABILITY_ID)) stop_field("capability_id", "identity mismatch")
  if (!identical(recipe$method_version, METHOD_VERSION)) stop_field("method_version", "identity mismatch")
  if (!is.character(recipe$scenario_identity) || length(recipe$scenario_identity) != 1L ||
      !grepl("^[A-Za-z0-9_.-]{1,80}$", recipe$scenario_identity)) {
    stop_field("scenario_identity", "must be a stable ASCII identifier")
  }

  design <- recipe$design
  expected_design <- sort(c(
    "predictor_construct", "outcome_construct", "predictor_indicator_loadings",
    "outcome_indicator_loadings", "population_path", "exogenous_distribution",
    "structural_disturbance_distribution", "indicator_error_distribution", "missing_data"
  ))
  if (!identical(sort(names(design)), expected_design)) stop_field("design", "unknown or missing fields")
  if (!grepl("^[A-Za-z0-9_.-]{1,80}$", design$predictor_construct)) stop_field("design.predictor_construct", "invalid identity")
  if (!grepl("^[A-Za-z0-9_.-]{1,80}$", design$outcome_construct)) stop_field("design.outcome_construct", "invalid identity")
  if (identical(design$predictor_construct, design$outcome_construct)) stop_field("design.outcome_construct", "must differ from predictor")
  for (field in c("predictor_indicator_loadings", "outcome_indicator_loadings")) {
    values <- unlist(design[[field]], use.names = FALSE)
    if (length(values) < 3L || length(values) > 10L || any(!is.finite(values)) || any(values < 0.50 | values > 0.95)) {
      stop_field(paste0("design.", field), "requires 3-10 finite loadings from 0.50 through 0.95")
    }
  }
  if (length(design$population_path) != 1L || !is.finite(design$population_path) || abs(design$population_path) > 0.80) {
    stop_field("design.population_path", "must be finite and between -0.80 and 0.80")
  }
  for (field in c("exogenous_distribution", "structural_disturbance_distribution", "indicator_error_distribution")) {
    if (!identical(design[[field]], "standard_normal")) stop_field(paste0("design.", field), "v1 supports standard_normal only")
  }
  if (!identical(design$missing_data, "none")) stop_field("design.missing_data", "v1 supports none only")

  estimator <- recipe$estimator
  if (!identical(sort(names(estimator)), sort(c("weighting_scheme", "preprocessing", "tolerance", "max_iterations")))) {
    stop_field("estimator", "unknown or missing fields")
  }
  if (!identical(estimator$weighting_scheme, "path")) stop_field("estimator.weighting_scheme", "v1 supports path only")
  if (!identical(estimator$preprocessing, "standardized")) stop_field("estimator.preprocessing", "v1 supports standardized only")
  if (!is.finite(estimator$tolerance) || estimator$tolerance < 1e-10 || estimator$tolerance > 1e-3) stop_field("estimator.tolerance", "out of range")
  if (estimator$max_iterations < 100 || estimator$max_iterations > 10000) stop_field("estimator.max_iterations", "out of range")
  if (!identical(recipe$inference, "case_bootstrap_normal_reference_two_sided")) stop_field("inference", "unsupported")
  grid <- unlist(recipe$sample_size_grid, use.names = FALSE)
  if (length(grid) < 2L || length(grid) > 16L || any(grid < 30 | grid > 5000) || any(diff(grid) <= 0)) stop_field("sample_size_grid", "requires 2-16 strictly increasing integers from 30 through 5000")
  if (!is.finite(recipe$alpha) || recipe$alpha < 0.001 || recipe$alpha > 0.10) stop_field("alpha", "out of range")
  if (!is.finite(recipe$target_power) || recipe$target_power < 0.50 || recipe$target_power > 0.99) stop_field("target_power", "out of range")
  if (!is.finite(recipe$confidence_level) || recipe$confidence_level < 0.80 || recipe$confidence_level > 0.999) stop_field("confidence_level", "out of range")
  min_mc <- if (production_counts) 100L else 10L
  min_bootstrap <- if (production_counts) 99L else 9L
  if (recipe$monte_carlo_replicates < min_mc || recipe$monte_carlo_replicates > 10000) stop_field("monte_carlo_replicates", "out of range")
  if (recipe$bootstrap_replicates < min_bootstrap || recipe$bootstrap_replicates > 1999 || recipe$bootstrap_replicates %% 2 == 0) stop_field("bootstrap_replicates", "must be odd and in range")
  if (recipe$workers < 1 || recipe$workers > 64) stop_field("workers", "out of range")
  best_lower <- unname(wilson_interval(recipe$monte_carlo_replicates, recipe$monte_carlo_replicates, recipe$confidence_level)["lower"])
  if (best_lower + .Machine$double.eps < recipe$target_power) stop_field("monte_carlo_replicates", "Wilson lower bound cannot reach target_power even with all successes")
  fits_per_dataset <- 1 + recipe$bootstrap_replicates
  planned_datasets <- length(grid) * recipe$monte_carlo_replicates
  estimated_pls_fits <- planned_datasets * fits_per_dataset
  estimated_pls_case_fits <- sum(grid) * recipe$monte_carlo_replicates * fits_per_dataset
  if (estimated_pls_fits > MAX_ESTIMATED_PLS_FITS) stop_field("sample_size_grid", "estimated PLS workload exceeds the 250000-fit desktop execution limit")
  if (estimated_pls_case_fits > MAX_ESTIMATED_PLS_CASE_FITS) stop_field("sample_size_grid", "estimated case-fit workload exceeds the 100000000-row desktop execution limit")
  invisible(TRUE)
}

wilson_interval <- function(successes, trials, confidence_level) {
  if (trials <= 0 || successes < 0 || successes > trials || confidence_level <= 0 || confidence_level >= 1) stop("invalid Wilson inputs")
  proportion <- successes / trials
  z <- qnorm(1 - (1 - confidence_level) / 2)
  z2 <- z * z
  denominator <- 1 + z2 / trials
  center <- (proportion + z2 / (2 * trials)) / denominator
  half_width <- z * sqrt(proportion * (1 - proportion) / trials + z2 / (4 * trials * trials)) / denominator
  c(lower = max(0, center - half_width), upper = min(1, center + half_width))
}

# Stable domain separation for the independent R stream. This is not intended
# to reproduce the Rust or Python generator; it prevents scheduling-order seeds.
domain_seed <- function(recipe, sample_size, replicate_index, subdomain) {
  text <- paste(STREAM_DOMAIN, recipe$method_version, recipe$scenario_identity,
                format(recipe$master_seed, scientific = FALSE), sample_size,
                replicate_index, subdomain, sep = "\0")
  bytes <- utf8ToInt(text)
  state <- 2166136261 %% 2147483647
  for (value in bytes) state <- (state * 16777619 + value) %% 2147483647
  as.integer(max(1, state))
}

generate_dataset <- function(recipe, sample_size, replicate_index) {
  set.seed(domain_seed(recipe, sample_size, replicate_index, "generated_data"))
  beta <- recipe$design$population_path
  predictor <- rnorm(sample_size)
  outcome <- beta * predictor + sqrt(1 - beta^2) * rnorm(sample_size)
  x_loadings <- unlist(recipe$design$predictor_indicator_loadings, use.names = FALSE)
  y_loadings <- unlist(recipe$design$outcome_indicator_loadings, use.names = FALSE)
  x_errors <- matrix(rnorm(sample_size * length(x_loadings)), nrow = sample_size)
  y_errors <- matrix(rnorm(sample_size * length(y_loadings)), nrow = sample_size)
  x <- outer(predictor, x_loadings) + sweep(x_errors, 2, sqrt(1 - x_loadings^2), "*")
  y <- outer(outcome, y_loadings) + sweep(y_errors, 2, sqrt(1 - y_loadings^2), "*")
  list(x = x, y = y)
}

standardize_score <- function(value) {
  scale <- sd(value)
  if (!is.finite(scale) || scale <= .Machine$double.eps) stop("zero or non-finite score variance")
  as.numeric((value - mean(value)) / scale)
}

estimate_pls_path <- function(x, y, tolerance, max_iterations) {
  x <- scale(x, center = TRUE, scale = TRUE)
  y <- scale(y, center = TRUE, scale = TRUE)
  if (any(!is.finite(x)) || any(!is.finite(y))) stop("zero or non-finite indicator variance")
  wx <- rep(1 / sqrt(ncol(x)), ncol(x))
  wy <- rep(1 / sqrt(ncol(y)), ncol(y))
  converged <- FALSE
  for (iteration in seq_len(max_iterations)) {
    sx <- standardize_score(x %*% wx)
    sy <- standardize_score(y %*% wy)
    direction <- if (sum(sx * sy) >= 0) 1 else -1
    next_x <- as.numeric(crossprod(x, direction * sy))
    next_y <- as.numeric(crossprod(y, direction * sx))
    next_x <- next_x / sqrt(sum(next_x^2))
    next_y <- next_y / sqrt(sum(next_y^2))
    if (next_x[1] < 0) next_x <- -next_x
    if (next_y[1] < 0) next_y <- -next_y
    change <- max(abs(next_x - wx), abs(next_y - wy))
    wx <- next_x
    wy <- next_y
    if (change <= tolerance) {
      converged <- TRUE
      break
    }
  }
  sx <- standardize_score(x %*% wx)
  sy <- standardize_score(y %*% wy)
  coefficient <- sum(sx * sy) / sum(sx * sx)
  list(coefficient = coefficient, converged = converged)
}

bootstrap_target_test <- function(recipe, data, sample_size, replicate_index) {
  original <- estimate_pls_path(data$x, data$y, recipe$estimator$tolerance, recipe$estimator$max_iterations)
  if (!original$converged) stop("point estimate did not converge")
  set.seed(domain_seed(recipe, sample_size, replicate_index, "bootstrap_inference"))
  estimates <- numeric(0)
  for (index in seq_len(recipe$bootstrap_replicates)) {
    rows <- sample.int(sample_size, sample_size, replace = TRUE)
    attempt <- try(estimate_pls_path(data$x[rows, , drop = FALSE], data$y[rows, , drop = FALSE],
                                     recipe$estimator$tolerance, recipe$estimator$max_iterations), silent = TRUE)
    if (!inherits(attempt, "try-error") && attempt$converged && is.finite(attempt$coefficient)) estimates <- c(estimates, attempt$coefficient)
  }
  required <- max(2, ceiling(recipe$bootstrap_replicates * 0.90))
  if (length(estimates) < required) stop("insufficient usable bootstrap replicates")
  standard_error <- sd(estimates)
  if (!is.finite(standard_error) || standard_error <= .Machine$double.eps) stop("bootstrap standard error unavailable")
  statistic <- original$coefficient / standard_error
  list(estimate = original$coefficient, p_value = 2 * pnorm(-abs(statistic)))
}

execute_replicate <- function(recipe, sample_size, replicate_index) {
  attempt <- try({
    data <- generate_dataset(recipe, sample_size, replicate_index)
    bootstrap_target_test(recipe, data, sample_size, replicate_index)
  }, silent = TRUE)
  if (inherits(attempt, "try-error")) {
    return(list(sample_size = sample_size, replicate_index = replicate_index,
                successful = FALSE, converged = FALSE, target_estimate = NULL,
                p_value_two_sided = NULL, rejected = FALSE,
                failure_code = "reference_inference_failed",
                failure_message = as.character(attempt)))
  }
  list(sample_size = sample_size, replicate_index = replicate_index,
       successful = TRUE, converged = TRUE,
       target_estimate = attempt$estimate,
       p_value_two_sided = attempt$p_value,
       rejected = attempt$p_value <= recipe$alpha,
       failure_code = NULL, failure_message = NULL)
}

run_reference <- function(recipe, production_counts = TRUE) {
  validate_recipe(recipe, production_counts)
  outcomes <- list()
  for (sample_size in unlist(recipe$sample_size_grid, use.names = FALSE)) {
    for (replicate_index in 0:(recipe$monte_carlo_replicates - 1)) {
      outcomes[[length(outcomes) + 1L]] <- execute_replicate(recipe, sample_size, replicate_index)
    }
  }
  rows <- lapply(unlist(recipe$sample_size_grid, use.names = FALSE), function(sample_size) {
    selected <- Filter(function(item) item$sample_size == sample_size, outcomes)
    successful <- sum(vapply(selected, function(item) item$successful, logical(1)))
    rejections <- sum(vapply(selected, function(item) item$rejected, logical(1)))
    interval <- wilson_interval(rejections, recipe$monte_carlo_replicates, recipe$confidence_level)
    list(sample_size = sample_size,
         requested_replicates = recipe$monte_carlo_replicates,
         attempted_replicates = length(selected),
         successful_replicates = successful,
         failed_replicates = recipe$monte_carlo_replicates - successful,
         rejections = rejections,
         achieved_power = rejections / recipe$monte_carlo_replicates,
         confidence_lower = unname(interval["lower"]),
         confidence_upper = unname(interval["upper"]),
         qualifies = unname(interval["lower"]) >= recipe$target_power)
  })
  qualifying <- Filter(function(row) row$qualifies, rows)
  decision <- if (length(qualifying)) list(status = "reached", sample_size = qualifying[[1]]$sample_size) else list(status = "not_reached")
  list(report_kind = "pls_sample_size_power_independent_r_reference_v1",
       passed = all(vapply(rows, function(row) row$attempted_replicates == row$requested_replicates, logical(1))),
       feature_id = CAPABILITY_ID, method_version = METHOD_VERSION,
       stream_domain = STREAM_DOMAIN, failure_policy = FAILURE_POLICY,
       interval_method = INTERVAL_METHOD, inference_method = INFERENCE_METHOD,
       rows = rows, decision = decision, outcomes = outcomes)
}

fixture_recipe <- function(replicates = 20L, bootstrap = 9L) {
  list(schema_version = 1L, capability_id = CAPABILITY_ID,
       method_version = METHOD_VERSION, scenario_identity = "r_reference_signal",
       design = list(predictor_construct = "x", outcome_construct = "y",
                     predictor_indicator_loadings = c(0.8, 0.8, 0.8),
                     outcome_indicator_loadings = c(0.8, 0.8, 0.8),
                     population_path = 0.30,
                     exogenous_distribution = "standard_normal",
                     structural_disturbance_distribution = "standard_normal",
                     indicator_error_distribution = "standard_normal", missing_data = "none"),
       estimator = list(weighting_scheme = "path", preprocessing = "standardized",
                        tolerance = 1e-7, max_iterations = 3000L),
       inference = "case_bootstrap_normal_reference_two_sided",
       sample_size_grid = c(60L, 120L), alpha = 0.05, target_power = 0.80,
       confidence_level = 0.95, monte_carlo_replicates = replicates,
       bootstrap_replicates = bootstrap, master_seed = 20260813, workers = 1L)
}

self_test <- function() {
  interval <- wilson_interval(80, 100, 0.95)
  stopifnot(abs(interval["lower"] - 0.7111708343) < 1e-9)
  stopifnot(abs(interval["upper"] - 0.8666330667) < 1e-9)
  recipe <- fixture_recipe()
  validate_recipe(recipe, production_counts = FALSE)
  first <- generate_dataset(recipe, 60L, 0L)
  repeat <- generate_dataset(recipe, 60L, 0L)
  stopifnot(identical(first, repeat))
  result <- run_reference(recipe, production_counts = FALSE)
  stopifnot(result$passed, length(result$outcomes) == 40L)
  cat("pls_sample_size_power_reference.R self-test passed\n")
}

args <- commandArgs(trailingOnly = TRUE)
if (identical(args, "--self-test")) {
  self_test()
} else if (length(args) == 2L) {
  if (!requireNamespace("jsonlite", quietly = TRUE)) stop("jsonlite is required for recipe I/O")
  recipe <- jsonlite::fromJSON(args[[1]], simplifyVector = FALSE)
  report <- run_reference(recipe, production_counts = TRUE)
  jsonlite::write_json(report, args[[2]], pretty = TRUE, auto_unbox = TRUE, null = "null", digits = NA)
} else {
  cat("usage: Rscript pls_sample_size_power_reference.R --self-test\n")
  cat("   or: Rscript pls_sample_size_power_reference.R recipe.json output.json\n")
  quit(status = 2L)
}
