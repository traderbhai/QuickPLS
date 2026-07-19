# Development-only GPL reference runner. Never distribute cSEM with QuickPLS.
args <- commandArgs(trailingOnly = TRUE)
input <- if (length(args) >= 1) args[[1]] else "validation/fixtures/corporate_reputation.csv"
output_path <- if (length(args) >= 2) args[[2]] else "validation/results/htmt_csem_0_6_1.csv"

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

matrix_rows <- function(matrix, variant) {
  constructs <- rownames(matrix)
  do.call(rbind, lapply(constructs, function(row_name) {
    do.call(rbind, lapply(constructs, function(column_name) {
      value <- as.numeric(matrix[row_name, column_name])
      if (value == 0 && row_name != column_name) {
        value <- as.numeric(matrix[column_name, row_name])
      }
      data.frame(
        variant = variant,
        row = row_name,
        column = column_name,
        value = value
      )
    }))
  }))
}

htmt_plus <- calculateHTMT(result, .type_htmt = "htmt", .absolute = TRUE)$htmts
htmt_original <- calculateHTMT(result, .type_htmt = "htmt", .absolute = FALSE)$htmts

rows <- rbind(
  matrix_rows(htmt_plus, "htmt_plus"),
  matrix_rows(htmt_original, "htmt_original")
)
rows$csem_version <- as.character(packageVersion("cSEM"))
rows$input <- input
write.csv(rows, output_path, row.names = FALSE, quote = FALSE)
