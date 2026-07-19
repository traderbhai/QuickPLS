# Development-only GPL reference runner. Never distribute this library with QuickPLS.
args <- commandArgs(trailingOnly = TRUE)
input <- if (length(args) >= 1) args[[1]] else "validation/fixtures/simple_reflective.csv"
output_path <- if (length(args) >= 2) args[[2]] else NA

suppressPackageStartupMessages(library(cSEM))

data <- read.csv(input)
is_corporate <- all(c("COMP1", "COMP2", "COMP3", "LIKE1", "LIKE2", "CUSA1", "CUSA2", "CUSL1", "CUSL2") %in% names(data))

if (is_corporate) {
  model <- "
  satisfaction ~ comp + like
  loyalty ~ satisfaction
  comp <~ COMP1 + COMP2 + COMP3
  like <~ LIKE1 + LIKE2
  satisfaction <~ CUSA1 + CUSA2
  loyalty <~ CUSL1 + CUSL2
  "
  indicators <- c("COMP1", "COMP2", "COMP3", "LIKE1", "LIKE2", "CUSA1", "CUSA2", "CUSL1", "CUSL2")
  construct_for_indicator <- function(indicator) {
    if (indicator %in% c("COMP1", "COMP2", "COMP3")) {
      "comp"
    } else if (indicator %in% c("LIKE1", "LIKE2")) {
      "like"
    } else if (indicator %in% c("CUSA1", "CUSA2")) {
      "satisfaction"
    } else {
      "loyalty"
    }
  }
  path_specs <- list(
    c("comp", "satisfaction"),
    c("like", "satisfaction"),
    c("satisfaction", "loyalty")
  )
} else {
  model <- "
  y ~ x
  x <~ x1 + x2
  y <~ y1 + y2
  "
  indicators <- c("x1", "x2", "y1", "y2")
  construct_for_indicator <- function(indicator) {
    if (indicator %in% c("x1", "x2")) "x" else "y"
  }
  path_specs <- list(c("x", "y"))
}

run_variant <- function(label, modes, scheme = "path") {
  variant <- csem(data, model, .approach_weights = "PLS-PM", .PLS_weight_scheme_inner = scheme, .PLS_modes = modes, .disattenuate = FALSE, .iter_max = 3000, .tolerance = 1e-12)
  path_rows <- do.call(rbind, lapply(path_specs, function(path) {
    data.frame(
      variant = label,
      kind = "path",
      source = path[[1]],
      target = path[[2]],
      indicator = "",
      value = as.numeric(variant$Estimates$Path_estimates[path[[2]], path[[1]]])
    )
  }))
  loading_matrix <- variant$Estimates$Loading_estimates
  weight_matrix <- variant$Estimates$Weight_estimates
  indicator_rows <- do.call(rbind, lapply(indicators, function(indicator) {
    construct <- construct_for_indicator(indicator)
    rbind(
      data.frame(variant = label, kind = "loading", source = construct, target = "", indicator = indicator, value = as.numeric(loading_matrix[construct, indicator])),
      data.frame(variant = label, kind = "weight", source = construct, target = "", indicator = indicator, value = as.numeric(weight_matrix[construct, indicator]))
    )
  }))
  rbind(path_rows, indicator_rows)
}

if (is_corporate) {
  results <- run_variant("CORPORATE_PATH_MODE_A", list(comp = "modeA", like = "modeA", satisfaction = "modeA", loyalty = "modeA"))
} else {
  results <- rbind(
    run_variant("PATH_MODE_A", list(x = "modeA", y = "modeA")),
    run_variant("MODE_B", list(x = "modeB", y = "modeB")),
    run_variant("FACTOR", list(x = "modeA", y = "modeA"), "factorial"),
    run_variant("PCA", list(x = "PCA", y = "PCA"))
  )
}
results$csem_version <- as.character(packageVersion("cSEM"))
results$input <- input

if (is.na(output_path)) {
  print(results)
} else {
  write.csv(results, output_path, row.names = FALSE, quote = FALSE)
}
