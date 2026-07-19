args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 2) {
  stop("usage: Rscript moderation_r_reference.R INPUT.csv OUTPUT.json")
}

input <- args[[1]]
output <- args[[2]]

data <- read.csv(input, stringsAsFactors = FALSE)
complete <- data[complete.cases(data[, c("x", "m", "y")]), c("x", "m", "y")]
standardized <- as.data.frame(scale(complete, center = TRUE, scale = TRUE))
standardized$product <- as.numeric(scale(standardized$x * standardized$m, center = TRUE, scale = TRUE))

fit <- lm(y ~ x + m + product, data = standardized)
coefficients <- coef(fit)
path_xy <- unname(coefficients[["x"]])
path_my <- unname(coefficients[["m"]])
path_xmy <- unname(coefficients[["product"]])

json_number <- function(value) {
  if (!is.finite(value)) {
    stop("non-finite reference value")
  }
  format(value, digits = 17, scientific = FALSE)
}

content <- paste0(
  "{\n",
  "  \"runtime\": \"R base lm\",\n",
  "  \"used_observations\": ", nrow(standardized), ",\n",
  "  \"paths\": {\n",
  "    \"x->y\": ", json_number(path_xy), ",\n",
  "    \"m->y\": ", json_number(path_my), ",\n",
  "    \"xm->y\": ", json_number(path_xmy), "\n",
  "  },\n",
  "  \"simple_slopes\": {\n",
  "    \"-1\": ", json_number(path_xy - path_xmy), ",\n",
  "    \"0\": ", json_number(path_xy), ",\n",
  "    \"1\": ", json_number(path_xy + path_xmy), "\n",
  "  }\n",
  "}\n"
)

writeLines(content, output, useBytes = TRUE)
