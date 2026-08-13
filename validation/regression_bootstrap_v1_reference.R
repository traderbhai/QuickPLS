args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 3) {
  stop("usage: Rscript regression_bootstrap_v1_reference.R FIXTURE.csv OUTPUT.json REPLICATES")
}

if (!requireNamespace("boot", quietly = TRUE)) {
  stop("R package 'boot' is required for the external case-resampling comparison")
}
if (!requireNamespace("jsonlite", quietly = TRUE)) {
  stop("R package 'jsonlite' is required for the external reference report")
}

fixture <- normalizePath(args[[1]], winslash = "/", mustWork = TRUE)
output <- args[[2]]
replicates <- as.integer(args[[3]])
if (is.na(replicates) || replicates < 99L) {
  stop("REPLICATES must be an integer of at least 99")
}

data <- read.csv(fixture, na.strings = c("", "NA"))
data <- data[complete.cases(data[, c("y", "bin_y", "x", "z", "w")]), ]
terms <- c("intercept", "x", "z", "w")
confidence <- 0.95

ols_statistic <- function(values, indices) {
  fit <- try(stats::lm(y ~ x + z + w, data = values[indices, , drop = FALSE]), silent = TRUE)
  if (inherits(fit, "try-error")) return(rep(NA_real_, length(terms)))
  coefficients <- stats::coef(fit)
  if (length(coefficients) != length(terms) || any(!is.finite(coefficients))) {
    return(rep(NA_real_, length(terms)))
  }
  unname(coefficients)
}

logistic_statistic <- function(values, indices) {
  sample <- values[indices, , drop = FALSE]
  if (length(unique(sample$bin_y)) != 2L) return(rep(NA_real_, length(terms)))
  fit <- suppressWarnings(try(stats::glm(
    bin_y ~ x + z + w,
    data = sample,
    family = stats::binomial(link = "logit"),
    control = stats::glm.control(epsilon = 1e-10, maxit = 100L)
  ), silent = TRUE))
  if (inherits(fit, "try-error") || !isTRUE(fit$converged)) {
    return(rep(NA_real_, length(terms)))
  }
  coefficients <- stats::coef(fit)
  probabilities <- stats::fitted(fit)
  if (length(coefficients) != length(terms)
      || any(!is.finite(coefficients))
      || max(abs(coefficients)) > 100
      || min(probabilities) <= 1e-10
      || max(probabilities) >= 1 - 1e-10) {
    return(rep(NA_real_, length(terms)))
  }
  unname(coefficients)
}

type7 <- function(values, probability) {
  as.numeric(stats::quantile(values, probs = probability, type = 7, names = FALSE))
}

bca_interval <- function(bootstrap_values, original, jackknife_values) {
  if (length(bootstrap_values) < 2L || length(jackknife_values) < 3L) return(NULL)
  below <- sum(bootstrap_values < original)
  tied <- sum(bootstrap_values == original)
  probability <- (below + 0.5 * tied) / length(bootstrap_values)
  probability <- min(max(probability, 0.5 / length(bootstrap_values)), 1 - 0.5 / length(bootstrap_values))
  bias_correction <- stats::qnorm(probability)
  centered <- mean(jackknife_values) - jackknife_values
  sum_squares <- sum(centered ^ 2)
  if (!is.finite(sum_squares) || sum_squares <= .Machine$double.eps) return(NULL)
  acceleration <- sum(centered ^ 3) / (6 * sum_squares ^ 1.5)
  if (!is.finite(acceleration)) return(NULL)
  tail <- (1 - confidence) / 2
  adjusted <- function(nominal) {
    z <- stats::qnorm(nominal)
    denominator <- 1 - acceleration * (bias_correction + z)
    if (!is.finite(denominator) || abs(denominator) <= .Machine$double.eps) return(NA_real_)
    value <- stats::pnorm(bias_correction + (bias_correction + z) / denominator)
    min(max(value, 0), 1)
  }
  lower_probability <- adjusted(tail)
  upper_probability <- adjusted(1 - tail)
  if (!is.finite(lower_probability) || !is.finite(upper_probability) || lower_probability > upper_probability) {
    return(NULL)
  }
  list(
    available = TRUE,
    bias_correction = bias_correction,
    acceleration = acceleration,
    lower = type7(bootstrap_values, lower_probability),
    upper = type7(bootstrap_values, upper_probability)
  )
}

summarize_run <- function(run, statistic, logistic) {
  usable <- run$t[apply(run$t, 1, function(row) all(is.finite(row))), , drop = FALSE]
  jackknife <- t(vapply(seq_len(nrow(data)), function(omitted) {
    statistic(data, setdiff(seq_len(nrow(data)), omitted))
  }, numeric(length(terms))))
  jackknife_complete <- all(apply(jackknife, 1, function(row) all(is.finite(row))))
  rows <- lapply(seq_along(terms), function(index) {
    values <- usable[, index]
    point <- run$t0[[index]]
    coefficient_bca <- if (jackknife_complete) {
      bca_interval(values, point, jackknife[, index])
    } else {
      NULL
    }
    odds_ratio <- NULL
    if (logistic) {
      transformed <- exp(values)
      odds_bca <- if (jackknife_complete) {
        bca_interval(transformed, exp(point), exp(jackknife[, index]))
      } else {
        NULL
      }
      odds_ratio <- list(
        original = exp(point),
        percentile_lower = type7(transformed, 0.025),
        percentile_upper = type7(transformed, 0.975),
        bca = odds_bca
      )
    }
    list(
      term = terms[[index]],
      original = point,
      bootstrap_mean = mean(values),
      standard_error = stats::sd(values),
      percentile_lower = type7(values, 0.025),
      percentile_upper = type7(values, 0.975),
      bca = coefficient_bca,
      odds_ratio = odds_ratio
    )
  })
  list(
    requested_replicates = replicates,
    usable_replicates = nrow(usable),
    failed_replicates = replicates - nrow(usable),
    jackknife_cases = nrow(data),
    usable_jackknife_cases = sum(apply(jackknife, 1, function(row) all(is.finite(row)))),
    coefficients = rows
  )
}

set.seed(20260815L)
ols_run <- boot::boot(data = data, statistic = ols_statistic, R = replicates, sim = "ordinary")
set.seed(20260816L)
logistic_run <- boot::boot(data = data, statistic = logistic_statistic, R = replicates, sim = "ordinary")

ols <- summarize_run(ols_run, ols_statistic, FALSE)
logistic <- summarize_run(logistic_run, logistic_statistic, TRUE)
passed <- (
  ols$usable_replicates >= ceiling(0.9 * replicates)
  && logistic$usable_replicates >= ceiling(0.9 * replicates)
  && ols$usable_jackknife_cases == nrow(data)
  && logistic$usable_jackknife_cases == nrow(data)
)

payload <- list(
  schema_version = 1,
  target = "regression_bootstrap_v1_external_r_reference",
  passed = passed,
  implementation = "R boot ordinary case resampling with lm and binomial glm",
  r_version = R.version.string,
  boot_version = as.character(utils::packageVersion("boot")),
  seed_ols = 20260815L,
  seed_logistic = 20260816L,
  confidence_level = confidence,
  terms = terms,
  observations = nrow(data),
  ols = ols,
  logistic = logistic
)

dir.create(dirname(output), recursive = TRUE, showWarnings = FALSE)
writeLines(jsonlite::toJSON(payload, auto_unbox = TRUE, digits = 16, null = "null"), output)
