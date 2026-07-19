args <- commandArgs(trailingOnly = TRUE)
if (length(args) != 1) {
  stop("usage: Rscript studentized_supplied_reference.R OUTPUT.csv")
}

if (!requireNamespace("boot", quietly = TRUE)) {
  stop("R package 'boot' is required for the boot.ci comparison")
}

output <- args[[1]]
original <- 10.0
outer_se <- 2.0
confidence <- 0.80
theta_star <- c(8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 7.0, 10.5, 9.5, 11.5)
inner_se <- c(1.0, 1.2, 0.9, 1.1, 1.3, 1.0, 0.8, 1.4, 1.05, 0.95)
pivots <- (theta_star - original) / inner_se
alpha <- 1.0 - confidence
lower_pivot <- as.numeric(stats::quantile(pivots, probs = alpha / 2.0, type = 7, names = FALSE))
upper_pivot <- as.numeric(stats::quantile(pivots, probs = 1.0 - alpha / 2.0, type = 7, names = FALSE))
lower <- original - upper_pivot * outer_se
upper <- original - lower_pivot * outer_se

boot_object <- list(
  t0 = c(original, outer_se^2),
  t = cbind(theta_star, inner_se^2),
  R = length(theta_star),
  data = seq_along(theta_star),
  statistic = function(data, index) c(0, 0),
  sim = "ordinary",
  call = match.call()
)
class(boot_object) <- "boot"
boot_ci <- boot::boot.ci(boot_object, conf = confidence, type = "stud", index = c(1, 2))
boot_stud <- boot_ci$student

dir.create(dirname(output), recursive = TRUE, showWarnings = FALSE)
write.csv(
  data.frame(
    method = c("r_type7", "r_boot_ci_stud"),
    confidence = c(confidence, confidence),
    original = c(original, original),
    outer_standard_error = c(outer_se, outer_se),
    lower_pivot = c(lower_pivot, NA_real_),
    upper_pivot = c(upper_pivot, NA_real_),
    lower = c(lower, boot_stud[4]),
    upper = c(upper, boot_stud[5])
  ),
  output,
  row.names = FALSE
)
