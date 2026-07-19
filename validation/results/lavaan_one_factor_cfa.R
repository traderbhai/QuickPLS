
suppressPackageStartupMessages(library(lavaan))
data <- read.csv("D:\\QuickPLS\\validation\\results\\lavaan_one_factor_cfa.csv")
numeric_cols <- names(data)
data[numeric_cols] <- scale(data[numeric_cols])
model <- "x =~ x1 + x2 + x3"
fit <- sem(model, data=data, meanstructure=FALSE, std.lv=FALSE, auto.fix.first=TRUE,
           estimator="ML", missing="listwise", fixed.x=FALSE)
pe <- parameterEstimates(fit, standardized=TRUE)
fitm <- fitMeasures(fit, c("chisq","df","pvalue","cfi","tli","rmsea","srmr","aic","bic"))
rows <- lapply(seq_len(nrow(pe)), function(i) as.list(pe[i, c("lhs","op","rhs","est","se","z","pvalue","std.lv","std.all")]))
payload <- list(parameters=rows, fit=as.list(fitm), syntax=model)
writeLines(jsonlite::toJSON(payload, auto_unbox=TRUE, pretty=TRUE, na="null", digits=16), "D:\\QuickPLS\\validation\\results\\lavaan_one_factor_cfa_reference.json")
