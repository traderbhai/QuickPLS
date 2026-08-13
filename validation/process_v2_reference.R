#!/usr/bin/env Rscript

# Independent R reference for QuickPLS graph-defined PROCESS v2.  Base R is
# sufficient: equations use lm(), HC3 covariance is computed independently,
# and resampling uses R's Mersenne-Twister stream rather than QuickPLS' indexed
# stream.  This file is validation-only and is never bundled with QuickPLS.

args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 3) stop("usage: process_v2_reference.R fixture.csv output.json replicates")
fixture_path <- args[[1]]
output_path <- args[[2]]
replicates <- as.integer(args[[3]])
if (!requireNamespace("jsonlite", quietly = TRUE)) stop("validation-only package jsonlite is required")

raw <- read.csv(fixture_path, check.names = FALSE, na.strings = c("", "NA"))
variables <- c("X", "M1", "M2", "M3", "M4", "W", "B", "C", "Y")
data <- raw[complete.cases(raw[, variables]), variables]

profile <- function(values, scale = "continuous") {
  list(
    raw_mean = mean(values), raw_sample_sd = sd(values),
    raw_min = min(values), raw_max = max(values), scale = scale
  )
}
profiles <- lapply(data, profile)
profiles$W <- profile(data$W, "continuous")
profiles$B <- profile(data$B, "binary_0_1")

reference_condition_text <- paste0(
  "Continuous moderators are evaluated at their original complete-sample raw means ",
  "(coded 0); binary moderators are evaluated at 0."
)

centered <- function(name, values) {
  if (profiles[[name]]$scale == "binary_0_1") values else values - profiles[[name]]$raw_mean
}

equation_specs <- list(
  M1 = list(c("path:X->M1", "path", "X"), c("control:C", "control", "C")),
  M2 = list(c("path:M1->M2", "path", "M1"), c("control:C", "control", "C")),
  M3 = list(c("path:X->M3", "path", "X"), c("moderator:W", "moderator_main", "W"),
            c("interaction:X*W", "interaction", "X", "W"), c("control:C", "control", "C")),
  M4 = list(c("path:X->M4", "path", "X"), c("control:C", "control", "C")),
  Y = list(c("path:X->Y", "path", "X"), c("path:M2->Y", "path", "M2"),
           c("path:M3->Y", "path", "M3"), c("path:M4->Y", "path", "M4"),
           c("moderator:W", "moderator_main", "W"), c("moderator:B", "moderator_main", "B"),
           c("interaction:M4*B", "interaction", "M4", "B"),
           c("interaction:W*B", "interaction", "W", "B"),
           c("interaction:X*B", "interaction", "X", "B"), c("interaction:X*W", "interaction", "X", "W"),
           c("interaction:X*W*B", "interaction", "X", "W", "B"),
           c("control:C", "control", "C"))
)

hc3_scaled_residuals <- function(residual, leverage, outcome) {
  denominator <- 1 - leverage
  unstable <- which(!is.finite(denominator) | denominator <= 1e-12)
  if (length(unstable) > 0) {
    stop(
      sprintf(
        "high_leverage_hc3_instability|equation=equation:%s|case_index=%d|one_minus_h=%.17g",
        outcome, unstable[[1]] - 1, denominator[[unstable[[1]]]]
      ),
      call. = FALSE
    )
  }
  # No clamp is permitted: stable HC3 uses the exact (1 - h) denominator.
  residual / denominator
}

hc3_high_leverage_boundary_check <- function() {
  stable <- hc3_scaled_residuals(2, 1 - 2e-12, "stable")
  rejected <- vapply(c(1 - 1e-12, 1, NaN, Inf), function(value) {
    message <- tryCatch(
      {
        hc3_scaled_residuals(1, value, "unstable")
        ""
      },
      error = function(error) conditionMessage(error)
    )
    startsWith(message, "high_leverage_hc3_instability|")
  }, logical(1))
  isTRUE(all(rejected)) && isTRUE(all.equal(stable, 1e12, tolerance = 5e-5))
}

validate_hc3_covariance_diagonal <- function(covariance, outcome) {
  if (!is.matrix(covariance) || nrow(covariance) != ncol(covariance) || any(!is.finite(covariance))) {
    stop(
      sprintf("invalid_hc3_covariance|equation=equation:%s|reason=nonfinite_or_nonsquare_covariance", outcome),
      call. = FALSE
    )
  }
  diagonal <- diag(covariance)
  invalid <- which(!is.finite(diagonal) | diagonal <= 0)
  if (length(invalid) > 0) {
    stop(
      sprintf(
        "invalid_hc3_covariance|equation=equation:%s|term_index=%d|variance=%.17g",
        outcome, invalid[[1]] - 1, diagonal[[invalid[[1]]]]
      ),
      call. = FALSE
    )
  }
  diagonal
}

hc3_covariance_diagonal_boundary_check <- function() {
  valid <- validate_hc3_covariance_diagonal(matrix(c(1, 0.25, 0.25, 2), nrow = 2), "valid")
  rejected <- vapply(c(0, -1e-12, NaN, Inf), function(value) {
    message <- tryCatch(
      {
        validate_hc3_covariance_diagonal(matrix(value, nrow = 1), "invalid")
        ""
      },
      error = function(error) conditionMessage(error)
    )
    startsWith(message, "invalid_hc3_covariance|")
  }, logical(1))
  isTRUE(all.equal(valid, c(1, 2))) && isTRUE(all(rejected))
}

simple_slope_variance_boundary_check <- function() {
  validate <- function(variance) {
    if (!is.finite(variance) || variance <= 0) {
      stop(
        sprintf("degenerate_simple_slope_variance|moderation=moderation:validation|variance=%.17g", variance),
        call. = FALSE
      )
    }
    sqrt(variance)
  }
  if (!isTRUE(all.equal(validate(1), 1))) return(FALSE)
  rejected <- vapply(c(0, -1e-12, NaN, Inf), function(value) {
    message <- tryCatch({ validate(value); "" }, error = function(error) conditionMessage(error))
    startsWith(message, "degenerate_simple_slope_variance|")
  }, logical(1))
  isTRUE(all(rejected))
}

column_location_scale <- function(values) {
  location <- 0
  centered_sum_squares <- 0
  for (index in seq_along(values)) {
    value <- values[[index]]
    if (!is.finite(value)) return(NULL)
    delta <- value - location
    location <- location + delta / index
    centered_sum_squares <- centered_sum_squares + delta * (value - location)
  }
  scale <- sqrt(centered_sum_squares / length(values))
  if (!is.finite(location) || !is.finite(scale) || scale <= 0) return(NULL)
  list(location = location, scale = scale)
}

scale_aware_ols <- function(design, outcome_values, outcome) {
  rows <- nrow(design)
  columns <- ncol(design)
  if (rows != length(outcome_values) || rows <= columns || columns == 0 ||
      any(!is.finite(design)) || any(!is.finite(outcome_values)) ||
      !identical(as.numeric(design[, 1]), rep(1, rows))) {
    stop(paste("rank deficient", outcome), call. = FALSE)
  }
  centers <- rep(0, columns)
  scales <- rep(1, columns)
  normalized <- design
  if (columns > 1) for (column in 2:columns) {
    location_scale <- column_location_scale(design[, column])
    if (is.null(location_scale)) stop(paste("rank deficient", outcome), call. = FALSE)
    centers[[column]] <- location_scale$location
    scales[[column]] <- location_scale$scale
    normalized[, column] <- (design[, column] - centers[[column]]) / scales[[column]]
  }
  decomposition <- svd(normalized, nu = columns, nv = columns)
  maximum <- decomposition$d[[1]]
  minimum <- decomposition$d[[columns]]
  rank_tolerance <- maximum * max(rows, columns) * .Machine$double.eps * 100
  if (!is.finite(maximum) || !is.finite(minimum) || maximum <= 0 || minimum <= rank_tolerance) {
    stop(paste("rank deficient", outcome), call. = FALSE)
  }
  normalized_beta <- decomposition$v %*%
    ((t(decomposition$u) %*% outcome_values) / decomposition$d)
  fitted <- as.vector(normalized %*% normalized_beta)
  residual <- outcome_values - fitted
  normalized_xtx_inverse <- decomposition$v %*%
    diag(1 / decomposition$d^2, nrow = columns) %*% t(decomposition$v)
  leverage <- rowSums((normalized %*% normalized_xtx_inverse) * normalized)
  scaled <- hc3_scaled_residuals(residual, leverage, outcome)
  normalized_covariance <- normalized_xtx_inverse %*%
    crossprod(normalized * scaled) %*% normalized_xtx_inverse
  raw_transform <- matrix(0, nrow = columns, ncol = columns)
  raw_transform[1, 1] <- 1
  if (columns > 1) for (column in 2:columns) {
    raw_transform[1, column] <- -centers[[column]] / scales[[column]]
    raw_transform[column, column] <- 1 / scales[[column]]
  }
  beta <- as.vector(raw_transform %*% normalized_beta)
  covariance <- raw_transform %*% normalized_covariance %*% t(raw_transform)
  covariance <- (covariance + t(covariance)) / 2
  if (any(!is.finite(beta)) || any(!is.finite(covariance))) {
    stop(paste("nonfinite estimate", outcome), call. = FALSE)
  }
  list(
    beta = beta, fitted = fitted, residual = residual, covariance = covariance,
    minimum_singular_value = minimum, maximum_singular_value = maximum,
    rank_tolerance = rank_tolerance
  )
}

scale_aware_solver_boundary_check <- function() {
  rows <- 80
  x <- vapply(0:(rows - 1), function(index) index / 9 - 4, numeric(1))
  design <- cbind(1, x)
  outcome <- vapply(seq_along(x), function(index) {
    zero_index <- index - 1
    1.25 + 0.75 * x[[index]] + ((zero_index * 7) %% 13 - 6) / 100
  }, numeric(1))
  base <- scale_aware_ols(design, outcome, "Y")
  unit_scale <- 1e-9
  shift <- 4.5
  transformed <- scale_aware_ols(cbind(1, unit_scale * (x + shift)), outcome, "Y")
  perturbation <- vapply(0:(rows - 1), function(index) ((index * 11) %% 17 - 8) * 1e-14, numeric(1))
  rejected <- startsWith(tryCatch({
    scale_aware_ols(cbind(1, x, x + perturbation), outcome, "Y")
    ""
  }, error = function(error) conditionMessage(error)), "rank deficient")
  fitted_delta <- max(abs(base$fitted - transformed$fitted))
  slope_delta <- abs(transformed$beta[[2]] * unit_scale - base$beta[[2]])
  intercept_delta <- abs(transformed$beta[[1]] - (base$beta[[1]] - base$beta[[2]] * shift))
  covariance_delta <- abs(transformed$covariance[2, 2] * unit_scale^2 - base$covariance[2, 2])
  statistic_delta <- abs(
    transformed$beta[[2]] / sqrt(transformed$covariance[2, 2]) -
      base$beta[[2]] / sqrt(base$covariance[2, 2])
  )
  list(
    passed = fitted_delta <= 1e-10 && slope_delta <= 1e-10 &&
      intercept_delta <= 1e-9 && covariance_delta <= 1e-10 &&
      statistic_delta <= 1e-9 && rejected,
    normalization = "non_intercept_welford_mean_population_rms_v1",
    rank_rule = "s_min_gt_s_max_times_max_n_p_times_epsilon_times_100",
    fitted_maximum_absolute_difference = fitted_delta,
    slope_back_transform_absolute_difference = slope_delta,
    intercept_back_transform_absolute_difference = intercept_delta,
    covariance_back_transform_absolute_difference = covariance_delta,
    statistic_absolute_difference = statistic_delta,
    relative_collinearity_rejected = rejected
  )
}

johnson_neyman_coded_roots <- function(quadratic, linear, constant, coded_min, coded_max) {
  supplied <- c(quadratic, linear, constant, coded_min, coded_max)
  if (any(!is.finite(supplied)) || coded_min > coded_max) return(numeric())
  midpoint <- coded_min / 2 + coded_max / 2
  half_range <- coded_max / 2 - coded_min / 2
  if (!is.finite(midpoint) || !is.finite(half_range) || half_range <= 0) return(numeric())
  domain_quadratic <- quadratic * half_range^2
  domain_linear <- half_range * (2 * quadratic * midpoint + linear)
  domain_constant <- quadratic * midpoint^2 + linear * midpoint + constant
  if (any(!is.finite(c(domain_quadratic, domain_linear, domain_constant)))) return(numeric())
  coefficient_scale <- max(abs(c(domain_quadratic, domain_linear, domain_constant)))
  if (coefficient_scale == 0) return(numeric())
  a <- domain_quadratic / coefficient_scale
  b <- domain_linear / coefficient_scale
  c_value <- domain_constant / coefficient_scale
  coefficient_tolerance <- 64 * .Machine$double.eps
  normalized_roots <- numeric()
  if (abs(a) <= coefficient_tolerance) {
    if (abs(b) > coefficient_tolerance) normalized_roots <- -c_value / b
  } else {
    discriminant_left <- b^2
    discriminant_right <- 4 * a * c_value
    discriminant <- discriminant_left - discriminant_right
    discriminant_scale <- max(abs(discriminant_left), abs(discriminant_right), .Machine$double.xmin)
    discriminant_tolerance <- 64 * .Machine$double.eps * discriminant_scale
    if (discriminant >= -discriminant_tolerance) {
      square_root <- if (abs(discriminant) <= discriminant_tolerance) 0 else sqrt(discriminant)
      if (square_root == 0) {
        normalized_roots <- -b / (2 * a)
      } else {
        signed_root <- if (b < 0 || (b == 0 && isTRUE(1 / b < 0))) -square_root else square_root
        q <- -0.5 * (b + signed_root)
        normalized_roots <- if (q == 0) -b / (2 * a) else c(q / a, c_value / q)
      }
    }
  }
  domain_tolerance <- 128 * .Machine$double.eps
  normalized_roots <- normalized_roots[
    is.finite(normalized_roots) &
      normalized_roots >= -1 - domain_tolerance & normalized_roots <= 1 + domain_tolerance
  ]
  normalized_roots <- sort(pmin(pmax(normalized_roots, -1), 1))
  if (length(normalized_roots) > 1) {
    deduplicated <- normalized_roots[[1]]
    for (root in normalized_roots[-1]) {
      root_scale <- max(abs(tail(deduplicated, 1)), abs(root), 1)
      if (abs(root - tail(deduplicated, 1)) > 128 * .Machine$double.eps * root_scale) {
        deduplicated <- c(deduplicated, root)
      }
    }
    normalized_roots <- deduplicated
  }
  roots <- vapply(normalized_roots, function(root) {
    if (root <= 0) coded_min + half_range * (root + 1) else coded_max - half_range * (1 - root)
  }, numeric(1))
  range_scale <- max(abs(coded_min), abs(coded_max), abs(coded_max - coded_min), .Machine$double.xmin)
  range_tolerance <- 128 * .Machine$double.eps * range_scale
  roots <- roots[is.finite(roots) & roots >= coded_min - range_tolerance & roots <= coded_max + range_tolerance]
  roots <- pmin(pmax(roots, coded_min), coded_max)
  if (length(roots) == 2 && quadratic != 0) {
    mapped <- roots
    for (target in seq_len(2)) {
      other <- mapped[[3 - target]]
      denominator <- quadratic * other
      if (is.finite(denominator) && denominator != 0) {
        companion <- constant / denominator
        at_boundary <- roots[[target]] == coded_min || roots[[target]] == coded_max
        if (at_boundary && is.finite(companion) &&
            companion >= coded_min - range_tolerance && companion <= coded_max + range_tolerance) {
          roots[[target]] <- min(max(companion, coded_min), coded_max)
        }
      }
    }
  }
  roots <- roots[is.finite(roots) & roots >= coded_min - range_tolerance & roots <= coded_max + range_tolerance]
  sort(unique(roots))
}

johnson_neyman_variance <- function(v0, v1, v2, coded) {
  variance <- v0 + 2 * v1 * coded + v2 * coded^2
  if (is.finite(variance) && variance > 0) variance else NA_real_
}

johnson_neyman_variance_positive_across_range <- function(v0, v1, v2, coded_min, coded_max) {
  if (is.na(johnson_neyman_variance(v0, v1, v2, coded_min)) ||
      is.na(johnson_neyman_variance(v0, v1, v2, coded_max))) return(FALSE)
  if (v2 > 0) {
    vertex <- -v1 / v2
    if (vertex > coded_min && vertex < coded_max &&
        is.na(johnson_neyman_variance(v0, v1, v2, vertex))) return(FALSE)
  }
  TRUE
}

johnson_neyman_root_solver_boundary_check <- function() {
  base <- johnson_neyman_coded_roots(1, -1, -2, -3, 3)
  transformed_deltas <- numeric()
  transformed_lengths <- integer()
  for (scale in c(1e-10, 1e10)) {
    shift <- 7 * scale
    transformed <- johnson_neyman_coded_roots(
      1 / scale^2,
      -1 / scale - 2 * shift / scale^2,
      -2 + shift / scale + shift^2 / scale^2,
      shift - 3 * scale,
      shift + 3 * scale
    )
    transformed_lengths <- c(transformed_lengths, length(transformed))
    transformed_deltas <- c(transformed_deltas, if (length(transformed) == 2) {
      max(abs(transformed - c(shift - scale, shift + 2 * scale)))
    } else Inf)
  }
  exact_double <- johnson_neyman_coded_roots(1, -2, 1, 0, 2)
  resolvable_near_double <- johnson_neyman_coded_roots(1, -(2 + 1e-12), 1 + 1e-12, 0, 2)
  imbalanced <- johnson_neyman_coded_roots(1, -(1e12 + 1e-12), 1, 0, 2e12)
  list(
    passed = length(base) == 2 && max(abs(base - c(-1, 2))) <= 1e-12 &&
      identical(transformed_lengths, c(2L, 2L)) &&
      transformed_deltas[[1]] <= 1e-22 && transformed_deltas[[2]] <= 1e-2 &&
      length(exact_double) == 1 && abs(exact_double[[1]] - 1) <= 1e-12 &&
      length(resolvable_near_double) == 2 &&
      abs(resolvable_near_double[[1]] - 1) <= 1e-12 &&
      abs(resolvable_near_double[[2]] - (1 + 1e-12)) <= 1e-12 &&
      length(imbalanced) == 2 && abs(imbalanced[[1]] - 1e-12) <= 1e-24 &&
      abs(imbalanced[[2]] - 1e12) <= 1e-3,
    domain_normalization = "coded_range_to_minus_one_plus_one_v1",
    coefficient_tolerance_multiplier = 64,
    root_deduplication_tolerance_multiplier = 128,
    stable_quadratic_formula = "q_formula_v1",
    exact_double_root_count = length(exact_double),
    resolvable_near_double_root_count = length(resolvable_near_double)
  )
}

johnson_neyman_invalid_covariance_boundary_check <- function() {
  invalid <- !johnson_neyman_variance_positive_across_range(0, 0, 1, -1, 1) &&
    !johnson_neyman_variance_positive_across_range(1, 1.1, 1, -2, 2)
  valid <- johnson_neyman_variance_positive_across_range(1, 0.1, 0.25, -1, 1)
  list(
    passed = invalid && valid,
    reason_code = "invalid_hc3_covariance",
    message = paste0(
      "Johnson-Neyman conditional-effect variance must be finite and strictly positive ",
      "across the tested moderator range."
    ),
    variance_rule = "finite_and_strictly_positive_across_tested_range"
  )
}

validate_original_endogenous_outcomes <- function(frame) {
  for (outcome in c("Y", "M1", "M2", "M3", "M4")) {
    values <- frame[[outcome]]
    if (any(values == 0) && any(values == 1) && all(values == 0 | values == 1)) {
      stop(paste0("binary_process_equation_outcome|variable=", outcome, "|scope=original_complete_sample"), call. = FALSE)
    }
  }
  TRUE
}

binary_endogenous_outcome_boundary_check <- function() {
  base <- as.data.frame(lapply(variables, function(name) seq(0.1, 8, length.out = 80)))
  names(base) <- variables
  rejected <- character()
  for (outcome in c("M1", "Y")) {
    candidate <- base
    candidate[[outcome]] <- rep(c(0, 1), 40)
    message <- tryCatch({ validate_original_endogenous_outcomes(candidate); "" },
                        error = function(error) conditionMessage(error))
    if (startsWith(message, "binary_process_equation_outcome|")) rejected <- c(rejected, outcome)
  }
  list(
    passed = identical(rejected, c("M1", "Y")) && isTRUE(validate_original_endogenous_outcomes(base)),
    reason_code = "binary_process_equation_outcome",
    rejected_outcomes = rejected,
    original_sample_only = TRUE
  )
}

semantic_probe_levels <- function(profile, variable) {
  if (profile$scale == "binary_0_1") {
    return(list(raw = c(0, 1), tokens = c("binary_0", "binary_1")))
  }
  levels <- c(
    profile$raw_mean - profile$raw_sample_sd,
    profile$raw_mean,
    profile$raw_mean + profile$raw_sample_sd
  )
  if (any(!is.finite(levels)) || !(levels[[1]] < levels[[2]] && levels[[2]] < levels[[3]])) {
    stop(
      paste0(
        "collapsed_process_probe_grid|PROCESS continuous moderator ", variable,
        " does not have three distinct finite mean-minus-SD, mean, and mean-plus-SD probes in f64"
      ),
      call. = FALSE
    )
  }
  list(raw = levels, tokens = c("minus_1sd", "mean", "plus_1sd"))
}

collapsed_probe_grid_boundary_check <- function() {
  collapsed <- list(
    raw_mean = 9007199254740992,
    raw_sample_sd = 0.25,
    scale = "continuous"
  )
  message <- tryCatch({ semantic_probe_levels(collapsed, "W"); "" },
                      error = function(error) conditionMessage(error))
  list(
    passed = startsWith(message, "collapsed_process_probe_grid|"),
    reason_code = "collapsed_process_probe_grid",
    semantic_assignment = "canonical_grid_index_primary_outer_conditioning_inner"
  )
}

fit_equation <- function(outcome, specs, frame, frame_profiles) {
  design <- data.frame(intercept = rep(1, nrow(frame)), check.names = FALSE)
  ids <- c("intercept")
  variables_by_term <- list(character())
  for (spec in specs) {
    term_id <- spec[[1]]
    vars <- spec[-c(1, 2)]
    value <- rep(1, nrow(frame))
    for (variable in vars) {
      raw_value <- frame[[variable]]
      if (length(vars) > 1 && frame_profiles[[variable]]$scale != "binary_0_1") {
        raw_value <- raw_value - frame_profiles[[variable]]$raw_mean
      }
      value <- value * raw_value
    }
    design[[term_id]] <- value
    ids <- c(ids, term_id)
    variables_by_term[[length(variables_by_term) + 1]] <- vars
  }
  x <- as.matrix(design)
  y <- frame[[outcome]]
  fit <- scale_aware_ols(x, y, outcome)
  residual <- fit$residual
  covariance <- fit$covariance
  validate_hc3_covariance_diagonal(covariance, outcome)
  beta <- as.vector(fit$beta)
  names(beta) <- ids
  list(beta = beta, covariance = covariance, ids = ids, variables = variables_by_term,
       residual_df = nrow(x) - ncol(x))
}

make_profiles <- function(frame) {
  result <- lapply(frame, profile)
  result$W <- profile(frame$W, "continuous")
  result$B <- profile(frame$B, "binary_0_1")
  result
}

fit_graph <- function(frame, raw_probe_profiles = NULL) {
  p <- make_profiles(frame)
  if (is.null(raw_probe_profiles)) raw_probe_profiles <- p
  equations <- lapply(names(equation_specs), function(outcome) fit_equation(outcome, equation_specs[[outcome]], frame, p))
  names(equations) <- names(equation_specs)
  coef_vars <- function(outcome, vars) {
    equation <- equations[[outcome]]
    index <- which(vapply(equation$variables, function(value) identical(value, vars), logical(1)))
    if (length(index) != 1) stop(paste("missing coefficient", outcome, paste(vars, collapse = "*")))
    unname(equation$beta[[index]])
  }
  edge <- function(from, to, probes = list()) {
    result <- coef_vars(to, c(from))
    probe <- function(name) if (!is.null(probes[[name]])) probes[[name]] else 0
    if (from == "X" && to == "M3") result <- result + coef_vars(to, c("X", "W")) * probe("W")
    if (from == "M4" && to == "Y") result <- result + coef_vars(to, c("M4", "B")) * probe("B")
    if (from == "X" && to == "Y") {
      result <- result + coef_vars(to, c("X", "W")) * probe("W") +
        coef_vars(to, c("X", "B")) * probe("B") +
        coef_vars(to, c("X", "W", "B")) * probe("W") * probe("B")
    }
    result
  }
  path_effect <- function(path, probes = list()) {
    result <- 1
    for (index in seq_len(length(path) - 1)) result <- result * edge(path[[index]], path[[index + 1]], probes)
    result
  }
  paths <- list(c("X", "M1", "M2", "Y"), c("X", "M3", "Y"), c("X", "M4", "Y"))
  reference_ids <- c("direct:X->Y", vapply(paths, function(path) paste0("indirect:", paste(path, collapse = "->")), character(1)), "total_indirect:X->Y", "total:X->Y")
  indirect <- vapply(paths, path_effect, numeric(1))
  reference_values <- c(edge("X", "Y"), indirect, sum(indirect), edge("X", "Y") + sum(indirect))

  probe_raw <- function(name) {
    source <- raw_probe_profiles[[name]]
    semantic_probe_levels(source, name)$raw
  }
  coded <- function(name, raw_value) if (p[[name]]$scale == "binary_0_1") raw_value else raw_value - p[[name]]$raw_mean
  conditional_ids <- character()
  conditional_values <- numeric()
  for (probe_index in seq_along(probe_raw("W"))) {
    raw_w <- probe_raw("W")[[probe_index]]
    suffix <- paste0("W=", c("minus_1sd", "mean", "plus_1sd")[[probe_index]])
    conditional_ids <- c(conditional_ids, paste0("indirect:X->M3->Y@", suffix))
    conditional_values <- c(conditional_values, path_effect(c("X", "M3", "Y"), list(W = coded("W", raw_w))))
  }
  for (probe_index in seq_along(probe_raw("B"))) {
    raw_b <- probe_raw("B")[[probe_index]]
    suffix <- paste0("B=", c("binary_0", "binary_1")[[probe_index]])
    conditional_ids <- c(conditional_ids, paste0("indirect:X->M4->Y@", suffix))
    conditional_values <- c(conditional_values, path_effect(c("X", "M4", "Y"), list(B = raw_b)))
  }
  index_ids <- c("index:X->M3->Y:X->M3:W", "index:X->M4->Y:M4->Y:B")
  index_values <- c(coef_vars("M3", c("X", "W")) * coef_vars("Y", c("M3")),
                    coef_vars("Y", c("M4", "B")) * coef_vars("M4", c("X")))

  slope_ids <- character()
  slope_values <- numeric()
  for (probe_index in seq_along(probe_raw("W"))) {
    raw_w <- probe_raw("W")[[probe_index]]
    suffix <- paste0("W=", c("minus_1sd", "mean", "plus_1sd")[[probe_index]])
    slope_ids <- c(slope_ids, paste0("slope:moderation:X->M3@W@", suffix))
    slope_values <- c(slope_values, edge("X", "M3", list(W = coded("W", raw_w))))
  }
  for (w_index in seq_along(probe_raw("W"))) for (b_index in seq_along(probe_raw("B"))) {
    raw_w <- probe_raw("W")[[w_index]]
    raw_b <- probe_raw("B")[[b_index]]
    suffix <- paste0("W=", c("minus_1sd", "mean", "plus_1sd")[[w_index]], ",B=", c("binary_0", "binary_1")[[b_index]])
    slope_ids <- c(slope_ids, paste0("slope:moderation:X->Y@W|B@", suffix))
    slope_values <- c(slope_values, edge("X", "Y", list(W = coded("W", raw_w), B = raw_b)))
  }
  for (probe_index in seq_along(probe_raw("B"))) {
    raw_b <- probe_raw("B")[[probe_index]]
    suffix <- paste0("B=", c("binary_0", "binary_1")[[probe_index]])
    slope_ids <- c(slope_ids, paste0("slope:moderation:M4->Y@B@", suffix))
    slope_values <- c(slope_values, edge("M4", "Y", list(B = raw_b)))
  }
  list(ids = c(reference_ids, conditional_ids, index_ids, slope_ids),
       values = c(reference_values, conditional_values, index_values, slope_values),
       equations = equations, profiles = p)
}

validate_original_endogenous_outcomes(data)
original <- fit_graph(data)
scale_aware_boundary <- scale_aware_solver_boundary_check()
jn_root_boundary <- johnson_neyman_root_solver_boundary_check()
jn_covariance_boundary <- johnson_neyman_invalid_covariance_boundary_check()
binary_outcome_boundary <- binary_endogenous_outcome_boundary_check()
collapsed_probe_boundary <- collapsed_probe_grid_boundary_check()
numerical_boundaries <- list(
  passed = scale_aware_boundary$passed && jn_root_boundary$passed &&
    jn_covariance_boundary$passed && binary_outcome_boundary$passed &&
    collapsed_probe_boundary$passed,
  scale_aware_solver = scale_aware_boundary,
  johnson_neyman_root_solver = jn_root_boundary,
  johnson_neyman_invalid_covariance = jn_covariance_boundary,
  binary_endogenous_outcome = binary_outcome_boundary,
  collapsed_probe_grid = collapsed_probe_boundary
)
reference_condition <- list(
  passed = length(original$ids) == 24 && all(is.finite(original$values)) &&
    isTRUE(all.equal(centered("W", profiles$W$raw_mean), 0)) &&
    isTRUE(all.equal(centered("B", 0), 0)),
  column = "Reference condition",
  value = reference_condition_text,
  continuous_coded_value = 0,
  binary_raw_value = 0
)
set.seed(20260814, kind = "Mersenne-Twister", normal.kind = "Inversion")
samples <- matrix(NA_real_, nrow = replicates, ncol = length(original$ids))
usable <- logical(replicates)
for (replicate in seq_len(replicates)) {
  indices <- sample.int(nrow(data), nrow(data), replace = TRUE)
  # Refit/center on this resample, then evaluate at the original raw probe
  # values so IDs and scientific conditioning points remain invariant.
  fitted <- tryCatch(fit_graph(data[indices, , drop = FALSE], original$profiles), error = function(error) NULL)
  if (!is.null(fitted) && identical(fitted$ids, original$ids) && all(is.finite(fitted$values))) {
    samples[replicate, ] <- fitted$values
    usable[[replicate]] <- TRUE
  }
}
samples <- samples[usable, , drop = FALSE]
type7 <- function(values, probability) as.numeric(quantile(values, probs = probability, type = 7, names = FALSE))
resampling <- list(
  estimand_ids = original$ids,
  original = as.list(original$values),
  requested_replicates = replicates,
  usable_replicates = nrow(samples),
  mean = as.list(colMeans(samples)),
  standard_error = as.list(apply(samples, 2, sd)),
  percentile_lower = as.list(apply(samples, 2, type7, probability = 0.025)),
  percentile_upper = as.list(apply(samples, 2, type7, probability = 0.975))
)

equation_payload <- lapply(names(original$equations), function(name) {
  fit <- original$equations[[name]]
  list(outcome = name, term_ids = fit$ids, estimates = as.list(unname(fit$beta)),
       coefficient_covariance = lapply(seq_len(nrow(fit$covariance)), function(index) {
         as.list(unname(fit$covariance[index, ]))
       }))
})

# The Python reference compares complete graph arithmetic.  R reports its
# independently fitted equation identities/coefficients plus all estimands;
# the promotion gate binds both layers rather than pretending R generated the
# QuickPLS result schema.
graph <- list(
  equation_term_ids = lapply(equation_payload, function(row) row$term_ids),
  equation_estimates = lapply(equation_payload, function(row) row$estimates),
  equation_covariances = lapply(equation_payload, function(row) row$coefficient_covariance),
  estimand_ids = original$ids,
  estimand_values = as.list(original$values)
)
report <- list(
  schema_version = 1,
  target = "process_v2_independent_r_reference",
  method_version = "regression_process_v2",
  passed = nrow(samples) >= ceiling(replicates * 0.9) && all(is.finite(original$values)) &&
    hc3_high_leverage_boundary_check() && hc3_covariance_diagonal_boundary_check() &&
    simple_slope_variance_boundary_check() && numerical_boundaries$passed &&
    reference_condition$passed,
  hc3_high_leverage_boundary = list(
    passed = hc3_high_leverage_boundary_check(),
    tolerance = 1e-12,
    comparison = "one_minus_h_less_than_or_equal",
    clamp = FALSE,
    reason_code = "high_leverage_hc3_instability"
  ),
  hc3_covariance_diagonal_boundary = list(
    passed = hc3_covariance_diagonal_boundary_check(),
    rule = "finite_and_strictly_positive",
    absolute_value = FALSE,
    zero_clamp = FALSE,
    reason_code = "invalid_hc3_covariance"
  ),
  simple_slope_variance_boundary = list(
    passed = simple_slope_variance_boundary_check(),
    rule = "finite_and_strictly_positive",
    absolute_value = FALSE,
    zero_clamp = FALSE,
    reason_code = "degenerate_simple_slope_variance"
  ),
  numerical_boundaries = numerical_boundaries,
  reference_condition = reference_condition,
  complete_cases = nrow(data), omitted_cases = nrow(raw) - nrow(data),
  graph = graph, resampling = resampling,
  runtime_boundary = "R and jsonlite are validation-only dependencies and are not bundled."
)
jsonlite::write_json(report, output_path, pretty = TRUE, auto_unbox = TRUE, digits = 16, null = "null")
if (!isTRUE(report$passed)) quit(status = 1)
