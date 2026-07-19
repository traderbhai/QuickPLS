args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 2) {
  stop("usage: Rscript mediation_r_reference.R INPUT.csv OUTPUT.json")
}

input <- args[[1]]
output <- args[[2]]

data <- read.csv(input, stringsAsFactors = FALSE)
standardized <- as.data.frame(scale(data[, c("x", "m", "y")], center = TRUE, scale = TRUE))

path_xm <- unname(coef(lm(m ~ x, data = standardized))[["x"]])
path_my <- unname(coef(lm(y ~ m, data = standardized))[["m"]])
indirect_xy <- path_xm * path_my

json_number <- function(value) {
  if (!is.finite(value)) {
    stop("non-finite reference value")
  }
  format(value, digits = 17, scientific = FALSE)
}

content <- paste0(
  "{\n",
  "  \"runtime\": \"R base lm\",\n",
  "  \"paths\": {\n",
  "    \"x->m\": ", json_number(path_xm), ",\n",
  "    \"m->y\": ", json_number(path_my), "\n",
  "  },\n",
  "  \"mediation\": {\n",
  "    \"x->m\": {\"direct\": ", json_number(path_xm), ", \"indirect\": 0, \"total\": ", json_number(path_xm), ", \"variance_accounted_for\": 0},\n",
  "    \"m->y\": {\"direct\": ", json_number(path_my), ", \"indirect\": 0, \"total\": ", json_number(path_my), ", \"variance_accounted_for\": 0},\n",
  "    \"x->y\": {\"direct\": 0, \"indirect\": ", json_number(indirect_xy), ", \"total\": ", json_number(indirect_xy), ", \"variance_accounted_for\": 1}\n",
  "  }\n",
  "}\n"
)

writeLines(content, output, useBytes = TRUE)
