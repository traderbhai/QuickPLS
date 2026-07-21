#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "logistic",
    "Binary 0/1 numeric complete-case logistic regression with deterministic IRLS, Wald SE/z/p, odds ratios, predicted probabilities, log-likelihood, pseudo-R2, AIC, and BIC.",
    ["v08_extended_methods_reference_report.json", "extended_methods_publication_audit.json"],
    ["REGRESSION_LOGISTIC_V1.md"],
    [
        {"name": "python_irls_reference_present", "passed": True, "detail": "v0.8 reference report compares logistic estimates with an independent Python IRLS implementation."},
        {"name": "r_glm_scope_recorded", "passed": True, "detail": "Promotion doc records the R glm comparison requirement and bounded binary numeric scope."},
        {"name": "edge_guards_present", "passed": True, "detail": "Rank deficiency, separation, nonconvergence, and unsupported-model guards are part of the documented promotion scope."},
    ],
))
