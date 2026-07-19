# Development-only GPL reference runner. Never distribute cSEM with QuickPLS.
args <- commandArgs(trailingOnly = TRUE)
data_output <- if (length(args) >= 1) args[[1]] else "validation/results/assessment_published_satisfaction.csv"
metrics_output <- if (length(args) >= 2) args[[2]] else "validation/results/assessment_published_satisfaction_csem_0_6_1.csv"

suppressPackageStartupMessages(library(cSEM))

data(satisfaction)
write.csv(satisfaction, data_output, row.names = FALSE, quote = FALSE)

model <- "
EXPE ~ IMAG
QUAL ~ EXPE
VAL  ~ EXPE + QUAL
SAT  ~ IMAG + EXPE + QUAL + VAL
LOY  ~ IMAG + SAT

IMAG <~ imag1 + imag2 + imag3
EXPE <~ expe1 + expe2 + expe3
QUAL <~ qual1 + qual2 + qual3 + qual4 + qual5
VAL  <~ val1  + val2  + val3

SAT  =~ sat1  + sat2  + sat3  + sat4
LOY  =~ loy1  + loy2  + loy3  + loy4
"

result <- csem(
  .data = satisfaction,
  .model = model,
  .approach_weights = "PLS-PM",
  .PLS_weight_scheme_inner = "path",
  .disattenuate = FALSE,
  .iter_max = 3000,
  .tolerance = 1e-12
)

assessment <- assess(
  result,
  .quality_criterion = c("ave", "rho_C", "htmt", "r2", "r2_adj", "vif")
)

rows <- data.frame(
  metric = character(),
  target = character(),
  source = character(),
  variant = character(),
  value = numeric()
)

append_row <- function(metric, target, source, variant, value) {
  rows <<- rbind(rows, data.frame(
    metric = metric,
    target = target,
    source = source,
    variant = variant,
    value = as.numeric(value)
  ))
}

for (name in names(assessment$AVE)) {
  append_row("ave", name, "", "csem_assess", assessment$AVE[[name]])
}
for (name in names(assessment$RhoC)) {
  append_row("rho_c", name, "", "csem_assess", assessment$RhoC[[name]])
}
for (name in names(assessment$R2)) {
  append_row("r2", name, "", "csem_assess", assessment$R2[[name]])
}
for (name in names(assessment$R2_adj)) {
  append_row("r2_adj", name, "", "csem_assess", assessment$R2_adj[[name]])
}

for (target in rownames(assessment$VIF)) {
  for (source in colnames(assessment$VIF)) {
    value <- as.numeric(assessment$VIF[target, source])
    if (!is.na(value) && value != 0) {
      append_row("structural_vif", target, source, "csem_assess", value)
    }
  }
}

fornell <- assessment$`Fornell-Larcker`
if (!is.null(fornell)) {
  for (target in rownames(fornell)) {
    for (source in colnames(fornell)) {
      append_row("fornell_larcker", target, source, "csem_assess", fornell[target, source])
    }
  }
}

htmt <- assessment$HTMT$htmts
for (target in rownames(htmt)) {
  for (source in colnames(htmt)) {
    value <- as.numeric(htmt[target, source])
    if (value == 0 && target != source) {
      value <- as.numeric(htmt[source, target])
    }
    append_row("htmt_original", target, source, "csem_assess", value)
  }
}

append_row("srmr", "", "", "estimated", cSEM:::calculateSRMR(result, .saturated = FALSE))
append_row("d_uls", "", "", "estimated", cSEM:::calculateDL(result, .saturated = FALSE))
append_row("srmr", "", "", "saturated", cSEM:::calculateSRMR(result, .saturated = TRUE))
append_row("d_uls", "", "", "saturated", cSEM:::calculateDL(result, .saturated = TRUE))

f2 <- cSEM:::calculatef2(result)
for (target in rownames(f2)) {
  for (source in colnames(f2)) {
    value <- as.numeric(f2[target, source])
    if (!is.na(value) && value != 0) {
      append_row("f_squared", target, source, "csem_assess_probe", value)
    }
  }
}

rows$csem_version <- as.character(packageVersion("cSEM"))
rows$fixture <- "cSEM satisfaction README example"
write.csv(rows, metrics_output, row.names = FALSE, quote = FALSE)
