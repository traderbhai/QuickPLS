# Development-only GPL reference runner. Never distribute cSEM with QuickPLS.
args <- commandArgs(trailingOnly = TRUE)
data_output_path <- if (length(args) >= 1) args[[1]] else "validation/fixtures/csem_threecommonfactors.csv"
reference_output_path <- if (length(args) >= 2) args[[2]] else "validation/results/pls_csem_threecommonfactors_0_6_1.csv"

suppressPackageStartupMessages(library(cSEM))

data(threecommonfactors)
fixture <- as.data.frame(threecommonfactors)
write.csv(fixture, data_output_path, row.names = FALSE, quote = FALSE)

model <- "
eta2 ~ eta1
eta3 ~ eta1 + eta2

eta1 =~ y11 + y12 + y13
eta2 =~ y21 + y22 + y23
eta3 =~ y31 + y32 + y33
"

result <- csem(
  fixture,
  model,
  .approach_weights = "PLS-PM",
  .PLS_weight_scheme_inner = "path",
  .disattenuate = FALSE,
  .iter_max = 3000,
  .tolerance = 1e-12
)

path_matrix <- result$Estimates$Path_estimates
loading_matrix <- result$Estimates$Loading_estimates
weight_matrix <- result$Estimates$Weight_estimates

path_rows <- do.call(rbind, lapply(c("eta2", "eta3"), function(target) {
  sources <- names(which(!is.na(path_matrix[target, ])))
  sources <- sources[path_matrix[target, sources] != 0]
  do.call(rbind, lapply(sources, function(source) {
    data.frame(
      kind = "path",
      source = source,
      target = target,
      indicator = "",
      value = as.numeric(path_matrix[target, source])
    )
  }))
}))

indicator_map <- list(
  eta1 = c("y11", "y12", "y13"),
  eta2 = c("y21", "y22", "y23"),
  eta3 = c("y31", "y32", "y33")
)

indicator_rows <- do.call(rbind, unlist(lapply(names(indicator_map), function(construct) {
  lapply(indicator_map[[construct]], function(indicator) {
    rbind(
      data.frame(kind = "loading", source = construct, target = "", indicator = indicator, value = as.numeric(loading_matrix[construct, indicator])),
      data.frame(kind = "weight", source = construct, target = "", indicator = indicator, value = as.numeric(weight_matrix[construct, indicator]))
    )
  })
}), recursive = FALSE))

rows <- rbind(path_rows, indicator_rows)
rows$reference <- "cSEM threecommonfactors"
rows$csem_version <- as.character(packageVersion("cSEM"))
rows$input <- data_output_path
write.csv(rows, reference_output_path, row.names = FALSE, quote = FALSE)
