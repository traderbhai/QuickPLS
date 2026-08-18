#!/usr/bin/env Rscript

# Independent base-R arithmetic microreference for plsc_bootstrap_v1.
#
# This script intentionally does not implement the PLSc estimator, indexed RNG,
# or SHA-256 ledger contract.  It checks only sample standard error, Hyndman-Fan
# Type 7 percentiles, the two-sided standard-normal diagnostic, and BCa
# arithmetic for the frozen hand cases.  It is not qualification evidence.

tolerance <- 1e-12
close_enough <- function(actual, expected) {
  is.finite(actual) && abs(actual - expected) <= tolerance * max(1, abs(expected))
}

bootstrap_values <- as.numeric(1:9)
original <- 5.0
jackknife_values <- c(4.0, 4.5, 5.0, 5.5, 6.0)
confidence_level <- 0.95
tail_probability <- (1.0 - confidence_level) / 2.0

bootstrap_mean <- mean(bootstrap_values)
bootstrap_bias <- bootstrap_mean - original
bootstrap_se <- sd(bootstrap_values)
normal_t <- original / bootstrap_se
normal_p_two_sided <- 2.0 * pnorm(abs(normal_t), lower.tail = FALSE)
percentile <- quantile(
  bootstrap_values,
  probs = c(tail_probability, 1.0 - tail_probability),
  type = 7,
  names = FALSE
)

replicate_count <- length(bootstrap_values)
below <- sum(bootstrap_values < original)
tied <- sum(bootstrap_values == original)
bias_probability <- (below + 0.5 * tied) / replicate_count
bias_probability <- min(
  1.0 - 0.5 / replicate_count,
  max(0.5 / replicate_count, bias_probability)
)
bias_correction <- qnorm(bias_probability)
jackknife_centered <- mean(jackknife_values) - jackknife_values
sum_squares <- sum(jackknife_centered ^ 2)
acceleration <- sum(jackknife_centered ^ 3) / (6.0 * sum_squares ^ 1.5)

adjust_probability <- function(nominal) {
  z_value <- qnorm(nominal)
  denominator <- 1.0 - acceleration * (bias_correction + z_value)
  pnorm(bias_correction + (bias_correction + z_value) / denominator)
}

bca_probabilities <- c(
  adjust_probability(tail_probability),
  adjust_probability(1.0 - tail_probability)
)
bca <- quantile(
  bootstrap_values,
  probs = bca_probabilities,
  type = 7,
  names = FALSE
)

stopifnot(
  close_enough(bootstrap_mean, 5.0),
  close_enough(bootstrap_bias, 0.0),
  close_enough(bootstrap_se, 2.7386127875258306),
  close_enough(normal_t, 1.8257418583505538),
  close_enough(normal_p_two_sided, 0.06788915486182903),
  close_enough(percentile[1], 1.2),
  close_enough(percentile[2], 8.8),
  close_enough(bias_correction, 0.0),
  close_enough(acceleration, 0.0),
  close_enough(bca[1], 1.2),
  close_enough(bca[2], 8.8)
)

degenerate_bootstrap <- rep(2.0, 4)
degenerate_jackknife <- rep(2.0, 3)
degenerate_centered <- mean(degenerate_jackknife) - degenerate_jackknife
stopifnot(
  close_enough(sd(degenerate_bootstrap), 0.0),
  close_enough(quantile(degenerate_bootstrap, 0.025, type = 7), 2.0),
  sum(degenerate_centered ^ 2) <= .Machine$double.eps
)

cat("passed=true\n")
cat("fixture_only=true\n")
cat("qualification_evidence=false\n")
cat("method_version=plsc_bootstrap_v1\n")
