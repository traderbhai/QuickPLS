
data <- read.csv("D:\\QuickPLS\\validation\\results\\ols_promotion_fixture.csv")
fit <- lm(y ~ x + m + z, data=data)
x <- model.matrix(fit)
resid <- residuals(fit)
xtx_inv <- solve(t(x) %*% x)
h <- diag(x %*% xtx_inv %*% t(x))
scaled <- resid / (1 - pmin(h, 0.999))
meat <- t(x) %*% diag(as.numeric(scaled * scaled), nrow=length(scaled)) %*% x
vcov_hc3 <- xtx_inv %*% meat %*% xtx_inv
se <- sqrt(diag(vcov_hc3))
stat <- coef(fit) / se
df <- df.residual(fit)
p <- 2 * (1 - pt(abs(stat), df))
out <- list(
  version=R.version.string,
  terms=names(coef(fit)),
  coefficients=as.numeric(coef(fit)),
  se_hc3=as.numeric(se),
  statistic=as.numeric(stat),
  p_value=as.numeric(p),
  r_squared=summary(fit)$r.squared,
  adjusted_r_squared=summary(fit)$adj.r.squared
)
writeLines(jsonlite::toJSON(out, auto_unbox=TRUE, digits=16), "D:\\QuickPLS\\validation\\results\\ols_promotion_r_reference.json")
