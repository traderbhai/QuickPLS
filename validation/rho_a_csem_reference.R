# Development-only GPL reference runner. Never distribute cSEM with QuickPLS.
args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 2) {
  stop("usage: Rscript rho_a_csem_reference.R INPUT.csv OUTPUT.csv")
}

suppressPackageStartupMessages(library(cSEM))
data <- read.csv(args[[1]])
data <- data[complete.cases(data), , drop = FALSE]
model <- "
y ~ x
x =~ x1 + x2 + x3
y =~ y1 + y2 + y3
"

fit <- csem(
  data,
  model,
  .approach_weights = "PLS-PM",
  .PLS_weight_scheme_inner = "path",
  .PLS_modes = list(x = "modeA", y = "modeA"),
  .disattenuate = TRUE,
  .conv_criterion = "diff_absolute",
  .iter_max = 3000,
  .tolerance = 1e-12
)

indicator_cor <- fit$Estimates$Indicator_VCV
weights <- fit$Estimates$Weight_estimates
measurement <- fit$Information$Model$measurement
builtin <- calculateRhoC(
  fit,
  .model_implied = FALSE,
  .only_common_factors = TRUE,
  .weighted = TRUE
)

manual_rho_a <- function(construct) {
  indicators <- colnames(measurement)[measurement[construct, ] != 0]
  R <- indicator_cor[indicators, indicators, drop = FALSE]
  u <- as.numeric(weights[construct, indicators, drop = TRUE])
  score_variance <- drop(t(u) %*% R %*% u)
  w <- u / sqrt(score_variance)
  g <- drop(t(w) %*% w)
  A <- R - diag(diag(R))
  D <- tcrossprod(w) - diag(w^2)
  numerator <- drop(t(w) %*% A %*% w)
  denominator <- drop(t(w) %*% D %*% w)
  data.frame(
    construct = construct,
    score_variance = score_variance,
    weight_norm_squared = g,
    off_diagonal_numerator = numerator,
    off_diagonal_denominator = denominator,
    rho_a_manual = g^2 * numerator / denominator,
    rho_a_csem = unname(builtin[[construct]])
  )
}

output <- do.call(rbind, lapply(c("x", "y"), manual_rho_a))
if (any(!is.finite(as.matrix(output[, -1])))) {
  stop("cSEM rho_A reference produced a nonfinite value")
}
if (any(abs(output$rho_a_manual - output$rho_a_csem) > 1e-10)) {
  stop("manual Equation 3 and cSEM weighted empirical reliability disagree")
}
if (all(abs(output$rho_a_manual - 1) <= 1e-10)) {
  stop("suspicious all-one rho_A reference; check disattenuation and scaling")
}
output$csem_version <- as.character(packageVersion("cSEM"))
output$inner_weighting <- "path"
output$mode <- "modeA"
write.csv(output, args[[2]], row.names = FALSE, quote = FALSE)
