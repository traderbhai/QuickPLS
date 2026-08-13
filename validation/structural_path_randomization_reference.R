#!/usr/bin/env Rscript

# Independent base-R recomputation for structural-path randomization v1.
#
# Inputs are construct scores exported by the tested release CLI, a long-form
# ordered design manifest, and an explicit deterministic permutation-index
# matrix generated independently in Python. No QuickPLS production code is
# loaded. R is a validation-only dependency and is never bundled at runtime.

args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 4L) {
  stop("usage: structural_path_randomization_reference.R scores.csv design.csv indices.csv output.csv")
}

options(digits = 17)

scores_path <- args[[1L]]
design_path <- args[[2L]]
indices_path <- args[[3L]]
output_path <- args[[4L]]

scores <- read.csv(scores_path, stringsAsFactors = FALSE, check.names = FALSE)
design <- read.csv(design_path, stringsAsFactors = FALSE, check.names = FALSE)
indices <- read.csv(indices_path, stringsAsFactors = FALSE, check.names = FALSE)

if (!identical(scores$row_index, seq.int(0L, nrow(scores) - 1L))) {
  stop("construct score rows are not exact zero-based order")
}
parameter_ordinals <- sort(unique(design$parameter_ordinal))
if (!identical(parameter_ordinals, seq.int(0L, length(parameter_ordinals) - 1L))) {
  stop("design parameter ordinals are not contiguous")
}

fit_coefficients <- function(predictors, outcome, subject) {
  design_matrix <- cbind(intercept = 1.0, predictors)
  fit <- lm.fit(x = design_matrix, y = outcome)
  if (fit$rank != ncol(design_matrix) || any(!is.finite(fit$coefficients))) {
    stop(paste("rank-deficient or nonfinite regression:", subject))
  }
  list(coefficients = fit$coefficients[-1L], fitted = as.vector(design_matrix %*% fit$coefficients))
}

output_rows <- vector("list", length(parameter_ordinals))
for (parameter_ordinal in parameter_ordinals) {
  rows <- design[design$parameter_ordinal == parameter_ordinal, , drop = FALSE]
  rows <- rows[order(rows$predictor_ordinal), , drop = FALSE]
  if (!identical(rows$predictor_ordinal, seq.int(0L, nrow(rows) - 1L))) {
    stop("predictor ordinals are not contiguous")
  }
  if (length(unique(rows$parameter)) != 1L || length(unique(rows$target)) != 1L) {
    stop("design manifest fields differ within a parameter")
  }
  focal_positions <- which(rows$is_focal == 1L)
  if (length(focal_positions) != 1L) {
    stop("design must identify exactly one focal predictor")
  }
  parameter <- rows$parameter[[1L]]
  target <- rows$target[[1L]]
  predictor_names <- rows$predictor
  if (!all(c(target, predictor_names) %in% names(scores))) {
    stop("design references a missing construct score")
  }
  predictors <- as.matrix(scores[, predictor_names, drop = FALSE])
  storage.mode(predictors) <- "double"
  outcome <- as.numeric(scores[[target]])
  full <- fit_coefficients(predictors, outcome, paste("full", parameter))

  nuisance_positions <- setdiff(seq_len(ncol(predictors)), focal_positions)
  if (length(nuisance_positions) == 0L) {
    nuisance <- matrix(numeric(0), nrow = nrow(predictors), ncol = 0L)
  } else {
    nuisance <- predictors[, nuisance_positions, drop = FALSE]
  }
  nuisance_fit <- fit_coefficients(nuisance, outcome, paste("nuisance", parameter))
  residuals <- outcome - nuisance_fit$fitted

  parameter_indices <- indices[indices$parameter_ordinal == parameter_ordinal, , drop = FALSE]
  permutation_ids <- sort(unique(parameter_indices$permutation_index))
  expected_permutations <- unique(rows$permutations)
  if (length(expected_permutations) != 1L ||
      !identical(permutation_ids, seq.int(0L, expected_permutations[[1L]] - 1L))) {
    stop("permutation ordinals are incomplete")
  }
  coefficients <- numeric(length(permutation_ids))
  for (permutation_index in permutation_ids) {
    selected <- parameter_indices[
      parameter_indices$permutation_index == permutation_index,
      ,
      drop = FALSE
    ]
    selected <- selected[order(selected$row_index), , drop = FALSE]
    if (!identical(selected$row_index, seq.int(0L, nrow(scores) - 1L))) {
      stop("permutation rows are incomplete")
    }
    chosen <- as.integer(selected$selected_residual_index)
    if (!identical(sort(chosen), seq.int(0L, nrow(scores) - 1L))) {
      stop("permutation indices are not an exact zero-based bijection")
    }
    permuted_outcome <- nuisance_fit$fitted + residuals[chosen + 1L]
    permuted <- fit_coefficients(
      predictors,
      permuted_outcome,
      paste("permutation", parameter, permutation_index)
    )
    coefficients[[permutation_index + 1L]] <- permuted$coefficients[[focal_positions]]
  }
  stored_original <- unique(rows$stored_original)
  stored_exceedances <- unique(rows$stored_exceedances)
  stored_p_value <- unique(rows$stored_p_value)
  if (length(stored_original) != 1L || length(stored_exceedances) != 1L ||
      length(stored_p_value) != 1L) {
    stop("stored inference fields differ within a parameter")
  }
  exceedances <- sum(abs(coefficients) >= abs(stored_original[[1L]]))
  p_value_two_sided <- (exceedances + 1.0) / (length(coefficients) + 1.0)
  output_rows[[parameter_ordinal + 1L]] <- data.frame(
    parameter_ordinal = rep(parameter_ordinal, length(permutation_ids)),
    parameter = rep(parameter, length(permutation_ids)),
    permutation_index = permutation_ids,
    coefficient = coefficients,
    reproduced_original = rep(full$coefficients[[focal_positions]], length(permutation_ids)),
    exceedances = rep(exceedances, length(permutation_ids)),
    p_value_two_sided = rep(p_value_two_sided, length(permutation_ids)),
    stringsAsFactors = FALSE,
    check.names = FALSE
  )
}

output <- do.call(rbind, output_rows)
write.csv(output, output_path, row.names = FALSE, quote = TRUE, na = "")
