#!/usr/bin/env Rscript

# Independent base-R oracle for the bounded QuickPLS post-hoc technical
# minimum sample-size v2 contract. This is validation-only code: QuickPLS does
# not load R at runtime.

FORMULA_CONSTANT <- 2.486
FORMULA_ALPHA <- 0.05
FORMULA_POWER <- 0.80
SIGNIFICANCE_ALPHA <- 0.05
MAX_EXACT_INTEGER <- 9007199254740991

empty_text <- function(value) {
  is.na(value) || !nzchar(trimws(enc2utf8(as.character(value))))
}

identity_key <- function(source_id, target_id) {
  paste0(enc2utf8(source_id), "\u001f", enc2utf8(target_id))
}

utf8_hex <- function(value) {
  paste(sprintf("%02x", as.integer(charToRaw(enc2utf8(value)))), collapse = "")
}

blank_result <- function(status, eligible_count, analytical_sample_size,
                         inference_present = FALSE,
                         significant_count = NA_integer_, driver = NULL,
                         driver_p = NA_real_, required = NA_real_) {
  data.frame(
    method_version = "inverse_square_root_posthoc_v2",
    status = status,
    formula_constant = FORMULA_CONSTANT,
    formula_alpha = FORMULA_ALPHA,
    formula_power = FORMULA_POWER,
    formula_test_direction = "directional",
    selection_rule = "smallest_absolute_statistically_significant_structural_path",
    significance_source = if (inference_present) "pls_bootstrap_normal_reference_two_sided" else NA_character_,
    significance_alpha = if (inference_present) SIGNIFICANCE_ALPHA else NA_real_,
    eligible_path_count = as.integer(eligible_count),
    significant_path_count = as.integer(significant_count),
    driver_source_id = if (is.null(driver)) NA_character_ else as.character(driver$source_id),
    driver_target_id = if (is.null(driver)) NA_character_ else as.character(driver$target_id),
    driver_p_value_two_sided = as.numeric(driver_p),
    minimum_absolute_path_coefficient = if (is.null(driver)) NA_real_ else abs(as.numeric(driver$coefficient)),
    technically_required_sample_size = as.numeric(required),
    analytical_sample_size = as.integer(analytical_sample_size),
    meets_technical_requirement = if (is.na(required)) NA else analytical_sample_size >= required,
    stringsAsFactors = FALSE,
    check.names = FALSE
  )
}

evaluate_posthoc <- function(paths, inference = NULL, analytical_sample_size = 0L) {
  required_path_columns <- c("source_id", "target_id", "coefficient")
  if (!all(required_path_columns %in% names(paths))) {
    stop("paths input must contain source_id,target_id,coefficient")
  }
  paths <- paths[, required_path_columns, drop = FALSE]
  eligible_count <- nrow(paths)
  inference_present <- !is.null(inference)

  invalid_path <- FALSE
  if (eligible_count > 0L) {
    invalid_path <- any(vapply(paths$source_id, empty_text, logical(1))) ||
      any(vapply(paths$target_id, empty_text, logical(1))) ||
      any(!is.finite(as.numeric(paths$coefficient)))
  }
  path_keys <- if (eligible_count == 0L) character() else
    mapply(identity_key, paths$source_id, paths$target_id, USE.NAMES = FALSE)
  if (invalid_path || anyDuplicated(path_keys)) {
    return(blank_result("inference_incomplete", eligible_count,
                        analytical_sample_size, inference_present))
  }
  if (eligible_count == 0L) {
    return(blank_result("not_applicable_no_structural_path", 0L,
                        analytical_sample_size, FALSE))
  }
  if (!inference_present) {
    return(blank_result("inference_unavailable", eligible_count,
                        analytical_sample_size, FALSE))
  }

  required_inference_columns <- c("source_id", "target_id", "p_value_two_sided")
  if (!all(required_inference_columns %in% names(inference))) {
    stop("inference input must contain source_id,target_id,p_value_two_sided")
  }
  inference <- inference[, required_inference_columns, drop = FALSE]
  inference_keys <- if (nrow(inference) == 0L) character() else
    mapply(identity_key, inference$source_id, inference$target_id, USE.NAMES = FALSE)
  probabilities <- suppressWarnings(as.numeric(inference$p_value_two_sided))
  invalid_inference <- nrow(inference) != eligible_count ||
    any(vapply(inference$source_id, empty_text, logical(1))) ||
    any(vapply(inference$target_id, empty_text, logical(1))) ||
    anyDuplicated(inference_keys) ||
    !setequal(path_keys, inference_keys) ||
    any(!is.finite(probabilities)) ||
    any(probabilities < 0 | probabilities > 1)
  if (invalid_inference) {
    return(blank_result("inference_incomplete", eligible_count,
                        analytical_sample_size, TRUE))
  }

  probability_by_key <- setNames(probabilities, inference_keys)
  path_probabilities <- unname(probability_by_key[path_keys])
  significant_indices <- which(path_probabilities <= SIGNIFICANCE_ALPHA)
  if (length(significant_indices) == 0L) {
    return(blank_result("no_statistically_significant_path", eligible_count,
                        analytical_sample_size, TRUE, 0L))
  }

  candidates <- paths[significant_indices, , drop = FALSE]
  candidates$.probability <- path_probabilities[significant_indices]
  ordering <- order(
    abs(as.numeric(candidates$coefficient)),
    vapply(candidates$source_id, utf8_hex, character(1)),
    vapply(candidates$target_id, utf8_hex, character(1)),
    method = "radix"
  )
  driver <- candidates[ordering[1L], , drop = FALSE]
  driver_magnitude <- abs(as.numeric(driver$coefficient[[1L]]))
  driver_p <- as.numeric(driver$.probability[[1L]])
  if (driver_magnitude == 0) {
    return(blank_result("undefined_zero_path", eligible_count,
                        analytical_sample_size, TRUE,
                        length(significant_indices), driver, driver_p))
  }

  ratio <- FORMULA_CONSTANT / driver_magnitude
  if (!is.finite(ratio) || ratio > sqrt(MAX_EXACT_INTEGER)) {
    return(blank_result("exceeds_supported_integer_range", eligible_count,
                        analytical_sample_size, TRUE,
                        length(significant_indices), driver, driver_p))
  }
  required <- ceiling(ratio * ratio)
  blank_result("available", eligible_count, analytical_sample_size, TRUE,
               length(significant_indices), driver, driver_p, required)
}

parse_args <- function(args) {
  parsed <- list(paths = NULL, inference = NULL, output = NULL, n = 0L,
                 self_test = FALSE)
  index <- 1L
  while (index <= length(args)) {
    key <- args[[index]]
    if (key == "--self-test") {
      parsed$self_test <- TRUE
      index <- index + 1L
      next
    }
    if (index == length(args)) stop(paste("missing value for", key))
    value <- args[[index + 1L]]
    if (key == "--paths") parsed$paths <- value
    else if (key == "--inference") parsed$inference <- value
    else if (key == "--output") parsed$output <- value
    else if (key == "--analytical-n") parsed$n <- as.integer(value)
    else stop(paste("unknown argument", key))
    index <- index + 2L
  }
  parsed
}

self_test <- function() {
  published <- c(`0.10` = 619, `0.15` = 275, `0.20` = 155,
                 `0.25` = 99, `0.30` = 69, `0.40` = 39)
  for (coefficient in names(published)) {
    paths <- data.frame(source_id = "x", target_id = "y",
                        coefficient = as.numeric(coefficient))
    inference <- data.frame(source_id = "x", target_id = "y",
                            p_value_two_sided = 0.01)
    result <- evaluate_posthoc(paths, inference, 200L)
    stopifnot(result$status == "available")
    stopifnot(result$technically_required_sample_size == published[[coefficient]])
  }

  paths <- data.frame(
    source_id = c("z", "b", "a"), target_id = rep("outcome", 3L),
    coefficient = c(0.20, -0.10, 0.10)
  )
  inference <- data.frame(
    source_id = paths$source_id, target_id = paths$target_id,
    p_value_two_sided = c(0.01, 0.20, 0.05)
  )
  result <- evaluate_posthoc(paths, inference, 619L)
  stopifnot(result$driver_source_id == "a")
  stopifnot(result$driver_p_value_two_sided == 0.05)
  stopifnot(result$technically_required_sample_size == 619)
  stopifnot(isTRUE(result$meets_technical_requirement))
  reversed <- evaluate_posthoc(paths[3:1, ], inference[3:1, ], 619L)
  stopifnot(identical(result, reversed))

  no_inference <- evaluate_posthoc(paths, NULL, 619L)
  stopifnot(no_inference$status == "inference_unavailable")
  none <- inference
  none$p_value_two_sided <- 0.051
  stopifnot(evaluate_posthoc(paths, none)$status ==
              "no_statistically_significant_path")
  duplicate <- rbind(inference[1, ], inference[1, ], inference[3, ])
  stopifnot(evaluate_posthoc(paths, duplicate)$status == "inference_incomplete")
  stopifnot(evaluate_posthoc(paths[0, ], inference[0, ])$status ==
              "not_applicable_no_structural_path")
  zero <- data.frame(source_id = "x", target_id = "y", coefficient = 0)
  zero_p <- data.frame(source_id = "x", target_id = "y", p_value_two_sided = 0.01)
  stopifnot(evaluate_posthoc(zero, zero_p)$status == "undefined_zero_path")
  tiny <- zero
  tiny$coefficient <- 1e-300
  stopifnot(evaluate_posthoc(tiny, zero_p)$status ==
              "exceeds_supported_integer_range")
  cat("PASS: independent R posthoc technical sample-size v2 oracle\n")
}

if (sys.nframe() == 0L) {
  args <- parse_args(commandArgs(trailingOnly = TRUE))
  if (args$self_test) {
    self_test()
  } else {
    if (is.null(args$paths) || is.null(args$output)) {
      stop("usage: --paths PATH [--inference PATH] --analytical-n N --output PATH")
    }
    paths <- read.csv(args$paths, stringsAsFactors = FALSE,
                      na.strings = c("NA", "NaN"), check.names = FALSE)
    inference <- if (is.null(args$inference)) NULL else
      read.csv(args$inference, stringsAsFactors = FALSE,
               na.strings = c("NA", "NaN"), check.names = FALSE)
    result <- evaluate_posthoc(paths, inference, args$n)
    write.csv(result, args$output, row.names = FALSE, na = "")
  }
}
