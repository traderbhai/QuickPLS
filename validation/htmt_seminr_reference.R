# Development-only external reference runner. Never distribute seminr with QuickPLS.
args <- commandArgs(trailingOnly = TRUE)
input <- if (length(args) >= 1) args[[1]] else "validation/fixtures/corporate_reputation.csv"
output_path <- if (length(args) >= 2) args[[2]] else "validation/results/htmt_seminr_2_5_0.csv"

suppressPackageStartupMessages(library(seminr))

data <- read.csv(input)

measurement_model <- constructs(
  composite("comp", multi_items("COMP", 1:3), weights = mode_A),
  composite("like", multi_items("LIKE", 1:2), weights = mode_A),
  composite("satisfaction", multi_items("CUSA", 1:2), weights = mode_A),
  composite("loyalty", multi_items("CUSL", 1:2), weights = mode_A)
)
structural_model <- relationships(
  paths(from = c("comp", "like"), to = "satisfaction"),
  paths(from = "satisfaction", to = "loyalty")
)

model <- estimate_pls(
  data = data,
  measurement_model = measurement_model,
  structural_model = structural_model,
  inner_weights = path_weighting,
  maxIt = 3000,
  stopCriterion = 1e-12
)

htmt <- summary(model)$validity$htmt
constructs <- rownames(htmt)

rows <- do.call(rbind, lapply(constructs, function(row_name) {
  do.call(rbind, lapply(constructs, function(column_name) {
    value <- suppressWarnings(as.numeric(htmt[row_name, column_name]))
    if (is.na(value) && row_name != column_name) {
      value <- suppressWarnings(as.numeric(htmt[column_name, row_name]))
    }
    if (is.na(value) && row_name == column_name) {
      value <- 1
    }
    data.frame(
      variant = "htmt_plus",
      row = row_name,
      column = column_name,
      value = value
    )
  }))
}))

rows$seminr_version <- as.character(packageVersion("seminr"))
rows$input <- input
write.csv(rows, output_path, row.names = FALSE, quote = FALSE)
