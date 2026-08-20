#!/usr/bin/env Rscript

# Validation-only independent General SEM Rank 0 oracle.
#
# cSEM 0.6.1 supplies the independently maintained indicator-level PLS-PM
# stage-one implementation.  All structural/effect, simultaneous two-stage
# moderation, score-orientation, product-rescaling, and bootstrap-summary
# arithmetic below is implemented with base R.  Neither R nor cSEM is a
# QuickPLS runtime or distribution dependency.

args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 2) {
  stop("usage: Rscript general_sem_rank0_csem_oracle.R INPUT.json OUTPUT.json")
}

suppressPackageStartupMessages(library(cSEM))
suppressPackageStartupMessages(library(jsonlite))

input_path <- args[[1]]
output_path <- args[[2]]
request <- fromJSON(input_path, simplifyVector = FALSE)

exact_names <- function(value, expected, subject) {
  if (!is.list(value) || !setequal(names(value), expected) || length(names(value)) != length(expected)) {
    stop(paste0(subject, " fields are not exact"))
  }
}

exact_names(
  request,
  c("schema_version", "operation", "scenario_id", "columns", "blocks", "paths", "interactions", "effect_target", "bootstrap"),
  "request"
)
if (!identical(request$schema_version, 1L) && !identical(request$schema_version, 1)) {
  stop("schema_version must equal 1")
}
if (!(request$operation %in% c("mediation_point", "mediation_bootstrap", "moderation_point", "moderation_bootstrap"))) {
  stop("operation is unsupported")
}
if (!is.character(request$scenario_id) || length(request$scenario_id) != 1 || request$scenario_id == "") {
  stop("scenario_id is invalid")
}

finite_number <- function(value, subject) {
  numeric <- as.numeric(value)
  if (length(numeric) != 1 || !is.finite(numeric)) stop(paste0(subject, " must be finite"))
  numeric
}

sample_sd <- function(values) {
  if (length(values) < 2) stop("sample standard deviation requires two values")
  result <- sd(values)
  if (!is.finite(result) || result <= 1e-12) stop("sample variance is zero or nonfinite")
  result
}

standardize <- function(values) {
  center <- mean(values)
  scale <- sample_sd(values)
  as.numeric((values - center) / scale)
}

as_column_frame <- function(columns) {
  if (!is.list(columns) || length(columns) == 0 || is.null(names(columns)) || any(names(columns) == "")) {
    stop("columns must be a named nonempty object")
  }
  lengths <- vapply(columns, length, integer(1))
  if (length(unique(lengths)) != 1 || lengths[[1]] < 3) stop("columns must have one row count >= 3")
  converted <- lapply(names(columns), function(name) {
    numeric <- vapply(columns[[name]], function(value) {
      if (is.null(value)) return(NA_real_)
      candidate <- suppressWarnings(as.numeric(value))
      if (length(candidate) != 1 || !is.finite(candidate)) stop(paste0("column ", name, " contains nonfinite data"))
      candidate
    }, numeric(1))
    numeric
  })
  names(converted) <- names(columns)
  as.data.frame(converted, check.names = FALSE, stringsAsFactors = FALSE)
}

data <- as_column_frame(request$columns)
blocks <- request$blocks
paths <- request$paths
interactions <- request$interactions
if (!is.list(blocks) || length(blocks) == 0) stop("blocks must be nonempty")
if (!is.list(paths)) stop("paths must be a list")
if (!is.list(interactions)) stop("interactions must be a list")

# Canonical scientific order is identity-based, never authoring/declaration
# order. This mirrors the compiled-plan authority without importing product
# code and makes the independent implementation a meaningful metamorphic
# oracle rather than an order-sensitive second copy.
blocks <- lapply(blocks, function(block) {
  exact_names(block, c("construct_id", "indicator_ids", "mode"), "block")
  if (!(block$mode %in% c("A", "B"))) stop("block mode must be A or B")
  indicators <- sort(as.character(unlist(block$indicator_ids, use.names = FALSE)), method = "radix")
  if (length(indicators) == 0) stop("block indicators must be nonempty")
  block$indicator_ids <- as.list(indicators)
  block
})
blocks <- blocks[order(vapply(blocks, function(block) as.character(block$construct_id), character(1)), method = "radix")]
if (length(interactions) > 0) {
  for (index in seq_along(interactions)) {
    exact_names(interactions[[index]], c("interaction_id", "focal_id", "moderator_id", "outcome_id"), paste0("interactions[", index, "]"))
  }
  interactions <- interactions[order(vapply(interactions, function(row) as.character(row$interaction_id), character(1)), method = "radix")]
}

block_ids <- vapply(blocks, function(block) {
  as.character(block$construct_id)
}, character(1))
if (length(unique(block_ids)) != length(block_ids)) stop("construct IDs must be unique")

indicator_ids <- unlist(lapply(blocks, function(block) unlist(block$indicator_ids, use.names = FALSE)), use.names = FALSE)
indicator_ids <- as.character(indicator_ids)
if (length(unique(indicator_ids)) != length(indicator_ids)) stop("indicators must belong to one block")
if (any(!(indicator_ids %in% names(data)))) stop("an indicator column is missing")
complete <- data[complete.cases(data[, indicator_ids, drop = FALSE]), , drop = FALSE]
if (nrow(complete) < 3) stop("fewer than three complete observations remain")

path_sources <- character(0)
path_targets <- character(0)
if (length(paths) > 0) {
  for (index in seq_along(paths)) {
    path <- paths[[index]]
    exact_names(path, c("source_id", "target_id"), paste0("paths[", index, "]"))
    source <- as.character(path$source_id)
    target <- as.character(path$target_id)
    if (!(source %in% block_ids) || !(target %in% block_ids) || source == target) stop("path identity is invalid")
    path_sources <- c(path_sources, source)
    path_targets <- c(path_targets, target)
  }
}
path_keys <- paste(path_sources, path_targets, sep = "->")
if (length(unique(path_keys)) != length(path_keys)) stop("paths must be unique")
if (length(path_keys) > 0) {
  path_order <- order(path_targets, path_sources, method = "radix")
  path_sources <- path_sources[path_order]
  path_targets <- path_targets[path_order]
  path_keys <- path_keys[path_order]
}

measurement_lines <- vapply(blocks, function(block) {
  paste0(block$construct_id, " <~ ", paste(unlist(block$indicator_ids, use.names = FALSE), collapse = " + "))
}, character(1))
structural_lines <- character(0)
for (target in block_ids) {
  sources <- path_sources[path_targets == target]
  if (length(sources) > 0) structural_lines <- c(structural_lines, paste0(target, " ~ ", paste(sources, collapse = " + ")))
}
model_syntax <- paste(c(structural_lines, measurement_lines), collapse = "\n")
modes <- setNames(lapply(blocks, function(block) if (block$mode == "A") "modeA" else "modeB"), block_ids)

ols <- function(predictors, outcome, subject) {
  matrix <- as.matrix(predictors)
  if (nrow(matrix) != length(outcome) || nrow(matrix) <= ncol(matrix)) stop(paste0(subject, " is underidentified"))
  decomposition <- qr(matrix, tol = 1e-12)
  if (decomposition$rank != ncol(matrix)) stop(paste0(subject, " is rank deficient"))
  coefficients <- as.numeric(qr.coef(decomposition, outcome))
  if (any(!is.finite(coefficients))) stop(paste0(subject, " is nonfinite"))
  coefficients
}

fit_stage_one <- function(frame) {
  fit <- csem(
    frame,
    model_syntax,
    .approach_weights = "PLS-PM",
    .PLS_weight_scheme_inner = "path",
    .PLS_modes = modes,
    .disattenuate = FALSE,
    .iter_max = 3000,
    .tolerance = 1e-10
  )
  if (!isTRUE(fit$Information$Weight_info$Convergence_status)) stop("cSEM PLS-PM did not converge")
  scores <- as.matrix(fit$Estimates$Construct_scores[, block_ids, drop = FALSE])
  weights <- fit$Estimates$Weight_estimates[block_ids, indicator_ids, drop = FALSE]
  loadings <- fit$Estimates$Loading_estimates[block_ids, indicator_ids, drop = FALSE]
  for (block_index in seq_along(blocks)) {
    id <- block_ids[[block_index]]
    indicators <- as.character(unlist(blocks[[block_index]]$indicator_ids, use.names = FALSE))
    anchor <- as.numeric(weights[id, indicators[[1]]])
    if (!is.finite(anchor) || abs(anchor) <= 1e-12) stop(paste0("indeterminate score orientation for ", id))
    if (anchor < 0) {
      scores[, id] <- -scores[, id]
      weights[id, ] <- -weights[id, ]
      loadings[id, ] <- -loadings[id, ]
    }
    scores[, id] <- standardize(scores[, id])
  }
  list(
    scores = scores,
    weights = weights,
    loadings = loadings,
    iterations = as.integer(fit$Information$Weight_info$Number_iterations)
  )
}

align_stage_one <- function(fit, original, sampled_indices_zero_based) {
  scores <- fit$scores
  weights <- fit$weights
  loadings <- fit$loadings
  corrections <- 0L
  for (id in block_ids) {
    reference <- original$scores[as.integer(sampled_indices_zero_based) + 1L, id]
    association <- cov(reference, scores[, id])
    if (!is.finite(association) || abs(association) <= 1e-12) stop(paste0("indeterminate score sign for ", id))
    if (association < 0) {
      scores[, id] <- -scores[, id]
      weights[id, ] <- -weights[id, ]
      loadings[id, ] <- -loadings[id, ]
      corrections <- corrections + 1L
    }
  }
  list(scores = scores, weights = weights, loadings = loadings, iterations = fit$iterations, corrections = corrections)
}

structural_coefficients <- function(scores) {
  values <- list()
  for (target in block_ids) {
    sources <- path_sources[path_targets == target]
    if (length(sources) == 0) next
    coefficients <- ols(scores[, sources, drop = FALSE], scores[, target], paste0("structural equation ", target))
    for (index in seq_along(sources)) values[[paste(sources[[index]], target, sep = "->")]] <- coefficients[[index]]
  }
  values
}

directed_paths <- function(source, target) {
  result <- list()
  walk <- function(current, route) {
    if (current == target) {
      result[[length(result) + 1L]] <<- route
      return()
    }
    successors <- path_targets[path_sources == current]
    for (successor in successors) if (!(successor %in% route)) walk(successor, c(route, successor))
  }
  walk(source, c(source))
  result
}

mediation_values <- function(scores, source, target) {
  coefficients <- structural_coefficients(scores)
  routes <- directed_paths(source, target)
  values <- list()
  for (route in routes) {
    if (length(route) < 3) next
    relations <- paste(route[-length(route)], route[-1], sep = "->")
    value <- prod(vapply(relations, function(relation) finite_number(coefficients[[relation]], relation), numeric(1)))
    values[[paste0("specific:", paste(route, collapse = "->"))]] <- value
  }
  direct_id <- paste(source, target, sep = "->")
  direct <- if (!is.null(coefficients[[direct_id]])) coefficients[[direct_id]] else 0
  total_indirect <- if (length(values) == 0) 0 else sum(unlist(values, use.names = FALSE))
  values[[paste0("total_indirect:", direct_id)]] <- total_indirect
  values[[paste0("direct:", direct_id)]] <- direct
  values[[paste0("total:", direct_id)]] <- direct + total_indirect
  list(values = values, structural = coefficients)
}

moderation_values <- function(scores) {
  if (length(interactions) == 0) stop("moderation operation requires interactions")
  interaction_ids <- vapply(interactions, function(interaction) {
    as.character(interaction$interaction_id)
  }, character(1))
  if (length(unique(interaction_ids)) != length(interaction_ids)) stop("interaction IDs must be unique")
  direct <- list()
  beta <- list()
  gamma <- list()
  product_mean <- list()
  product_scale <- list()
  slopes <- list()
  outcomes <- unique(vapply(interactions, function(row) as.character(row$outcome_id), character(1)))
  for (outcome in outcomes) {
    sources <- path_sources[path_targets == outcome]
    selected <- interactions[vapply(interactions, function(row) as.character(row$outcome_id) == outcome, logical(1))]
    product_columns <- list()
    for (interaction in selected) {
      id <- as.character(interaction$interaction_id)
      focal <- as.character(interaction$focal_id)
      moderator <- as.character(interaction$moderator_id)
      required <- c(paste(focal, outcome, sep = "->"), paste(moderator, outcome, sep = "->"))
      if (any(!(required %in% path_keys))) stop("strong hierarchy path is missing")
      raw <- scores[, focal] * scores[, moderator]
      product_mean[[id]] <- mean(raw)
      product_scale[[id]] <- sample_sd(raw)
      product_columns[[id]] <- standardize(raw)
    }
    matrix <- cbind(scores[, sources, drop = FALSE], as.data.frame(product_columns, check.names = FALSE))
    coefficients <- ols(matrix, scores[, outcome], paste0("joint stage-two equation ", outcome))
    for (index in seq_along(sources)) direct[[paste(sources[[index]], outcome, sep = "->")]] <- coefficients[[index]]
    offset <- length(sources)
    for (index in seq_along(selected)) {
      interaction <- selected[[index]]
      id <- as.character(interaction$interaction_id)
      focal <- as.character(interaction$focal_id)
      coefficient <- coefficients[[offset + index]]
      beta[[id]] <- coefficient
      gamma[[id]] <- coefficient / product_scale[[id]]
      focal_coefficient <- direct[[paste(focal, outcome, sep = "->")]]
      slopes[[id]] <- unname(vapply(c(-1, 0, 1), function(probe) focal_coefficient + gamma[[id]] * probe, numeric(1)))
    }
  }
  list(
    structural = direct,
    standardized_product_coefficients = beta,
    scientific_gammas = gamma,
    product_means = product_mean,
    product_scales = product_scale,
    fixed_probe_slopes = slopes
  )
}

point <- fit_stage_one(complete)
effect_target <- request$effect_target
point_result <- if (grepl("^mediation", request$operation)) {
  exact_names(effect_target, c("source_id", "target_id"), "effect_target")
  mediation_values(point$scores, as.character(effect_target$source_id), as.character(effect_target$target_id))
} else {
  if (!is.null(effect_target)) stop("moderation effect_target must be null")
  moderation_values(point$scores)
}

bootstrap_result <- NULL
if (grepl("_bootstrap$", request$operation)) {
  bootstrap <- request$bootstrap
  exact_names(bootstrap, c("confidence_level", "replicate_indices"), "bootstrap")
  confidence <- finite_number(bootstrap$confidence_level, "confidence_level")
  if (!(confidence > 0 && confidence < 1)) stop("confidence_level must be in (0,1)")
  replicate_indices <- bootstrap$replicate_indices
  if (!is.list(replicate_indices) || length(replicate_indices) < 2 || length(replicate_indices) > 10000) stop("replicate_indices count is invalid")
  requested <- length(replicate_indices)
  originals <- if (grepl("^mediation", request$operation)) point_result$values else point_result$scientific_gammas
  distributions <- setNames(lapply(names(originals), function(id) numeric(0)), names(originals))
  failures <- list()
  usable_indices <- integer(0)
  sign_corrections <- 0L
  for (replicate_index in seq_along(replicate_indices)) {
    indices <- as.integer(unlist(replicate_indices[[replicate_index]], use.names = FALSE))
    if (length(indices) != nrow(complete) || any(indices < 0L) || any(indices >= nrow(complete))) stop("replicate index vector is invalid")
    outcome <- tryCatch({
      refit <- fit_stage_one(complete[indices + 1L, , drop = FALSE])
      aligned <- align_stage_one(refit, point, indices)
      values <- if (grepl("^mediation", request$operation)) {
        mediation_values(aligned$scores, as.character(effect_target$source_id), as.character(effect_target$target_id))$values
      } else {
        moderation_values(aligned$scores)$scientific_gammas
      }
      if (!setequal(names(values), names(originals))) stop("bootstrap target identities drifted")
      list(ok = TRUE, values = values, corrections = aligned$corrections)
    }, error = function(error) list(ok = FALSE, message = conditionMessage(error)))
    if (isTRUE(outcome$ok)) {
      for (id in names(originals)) distributions[[id]] <- c(distributions[[id]], finite_number(outcome$values[[id]], id))
      usable_indices <- c(usable_indices, replicate_index - 1L)
      sign_corrections <- sign_corrections + outcome$corrections
    } else {
      failures[[length(failures) + 1L]] <- list(replicate_index = replicate_index - 1L, reason = outcome$message)
    }
  }
  minimum_usable <- max(2L, as.integer(ceiling(0.9 * requested)))
  published <- length(usable_indices) >= minimum_usable
  summaries <- list()
  if (published) {
    alpha <- 1 - confidence
    for (id in names(originals)) {
      values <- distributions[[id]]
      original <- finite_number(originals[[id]], id)
      center <- mean(values)
      exceedances <- sum(abs(values - original) >= abs(original))
      summaries[[id]] <- list(
        original = original,
        mean = center,
        bias = center - original,
        standard_error = sample_sd(values),
        lower = unname(quantile(values, alpha / 2, type = 7, names = FALSE)),
        upper = unname(quantile(values, 1 - alpha / 2, type = 7, names = FALSE)),
        exceedances = as.integer(exceedances),
        plus_one_two_sided_probability = (exceedances + 1) / (length(values) + 1)
      )
    }
  }
  bootstrap_result <- list(
    requested = requested,
    usable = length(usable_indices),
    minimum_usable = minimum_usable,
    published = published,
    summaries = summaries,
    failures = failures,
    usable_indices = as.list(usable_indices),
    sign_corrections = sign_corrections
  )
} else {
  if (!is.null(request$bootstrap)) stop("point operation bootstrap must be null")
}

output <- list(
  schema_version = 1L,
  report_kind = "quickpls_general_sem_rank0_independent_csem_oracle",
  scenario_id = request$scenario_id,
  operation = request$operation,
  runtime = list(
    r_version = as.character(getRversion()),
    csem_version = as.character(packageVersion("cSEM")),
    stage_one = "cSEM PLS-PM",
    downstream = "independent base R"
  ),
  used_observations = nrow(complete),
  point = list(
    iterations = point$iterations,
    weights = lapply(block_ids, function(id) setNames(as.list(as.numeric(point$weights[id, as.character(unlist(blocks[[match(id, block_ids)]]$indicator_ids, use.names = FALSE))])), as.character(unlist(blocks[[match(id, block_ids)]]$indicator_ids, use.names = FALSE)))),
    loadings = lapply(block_ids, function(id) setNames(as.list(as.numeric(point$loadings[id, as.character(unlist(blocks[[match(id, block_ids)]]$indicator_ids, use.names = FALSE))])), as.character(unlist(blocks[[match(id, block_ids)]]$indicator_ids, use.names = FALSE)))),
    values = point_result
  ),
  bootstrap = bootstrap_result,
  qualification_ready = FALSE,
  independence = "cSEM 0.6.1 independently estimates indicator-level PLS-PM scores; base R independently recomputes every downstream target and summary"
)
names(output$point$weights) <- block_ids
names(output$point$loadings) <- block_ids
write_json(output, output_path, auto_unbox = TRUE, pretty = TRUE, digits = 17, null = "null", na = "null")
