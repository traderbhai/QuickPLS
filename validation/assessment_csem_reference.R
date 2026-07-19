# Development-only GPL reference runner. Never distribute cSEM with QuickPLS.
args <- commandArgs(trailingOnly = TRUE)
input <- if (length(args) >= 1) args[[1]] else "validation/fixtures/corporate_reputation.csv"
output_path <- if (length(args) >= 2) args[[2]] else "validation/results/assessment_csem_0_6_1.csv"

suppressPackageStartupMessages(library(cSEM))

data <- read.csv(input)
model <- "
satisfaction ~ comp + like
loyalty ~ satisfaction

comp =~ COMP1 + COMP2 + COMP3
like =~ LIKE1 + LIKE2
satisfaction =~ CUSA1 + CUSA2
loyalty =~ CUSL1 + CUSL2
"

result <- csem(
  data,
  model,
  .approach_weights = "PLS-PM",
  .PLS_weight_scheme_inner = "path",
  .disattenuate = FALSE,
  .iter_max = 3000,
  .tolerance = 1e-12
)

assessment <- assess(result, .quality_criterion = c("r2", "r2_adj", "f2", "vif"))
values <- assessment
rows <- data.frame(
  metric = character(),
  target = character(),
  source = character(),
  variant = character(),
  value = numeric(),
  stringsAsFactors = FALSE
)

append_row <- function(metric, target, source, variant, value) {
  rows <<- rbind(rows, data.frame(
    metric = metric,
    target = target,
    source = source,
    variant = variant,
    value = as.numeric(value),
    stringsAsFactors = FALSE
  ))
}

for (name in names(values$R2)) {
  append_row("r2", name, "", "csem_assess", values$R2[[name]])
}

for (name in names(values$R2_adj)) {
  append_row("r2_adj", name, "", "csem_assess", values$R2_adj[[name]])
}

if (!is.null(values$VIF)) {
  for (target in rownames(values$VIF)) {
    for (source in colnames(values$VIF)) {
      value <- values$VIF[target, source]
      if (!is.na(value) && is.finite(value)) {
        append_row("structural_vif", target, source, "csem_assess", value)
      }
    }
  }
}

if (!is.null(values$F2)) {
  for (target in rownames(values$F2)) {
    for (source in colnames(values$F2)) {
      value <- values$F2[target, source]
      if (!is.na(value) && is.finite(value) && value != 0) {
        append_row("f_squared", target, source, "csem_assess_probe", value)
      }
    }
  }
}

append_row("d_uls", "", "", "estimated", calculateDL(result, .saturated = FALSE))
append_row("d_uls", "", "", "saturated", calculateDL(result, .saturated = TRUE))
append_row("srmr", "", "", "estimated", calculateSRMR(result, .saturated = FALSE))
append_row("srmr", "", "", "saturated", calculateSRMR(result, .saturated = TRUE))

rows$csem_version <- as.character(packageVersion("cSEM"))
rows$input <- input
write.csv(rows, output_path, row.names = FALSE, quote = FALSE)
