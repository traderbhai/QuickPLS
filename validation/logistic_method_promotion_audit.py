#!/usr/bin/env python3
from third_batch_promotion_common import audit_method


raise SystemExit(audit_method(
    "logistic",
    "Binary 0/1 numeric complete-case logistic regression with deterministic IRLS, Wald SE/z/p, odds ratios, predicted probabilities, log-likelihood, pseudo-R2, AIC, and BIC.",
    [
        {
            "name": "v08_logistic_reference_report.json",
            "required_true": ["checks.logistic.passed"],
            "required_values": {
                "schema_version": 2,
                "report_scope": "method_specific",
                "selected_section": "logistic",
                "checks.logistic.method_version": "regression_logistic_v1",
            },
            "source_paths": ["validation/v08_extended_methods_reference.py"],
            "companions": [{
                "path": "validation/results/v08_regression_logistic_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "regression",
                    "provenance.method_version": "regression_logistic_v1",
                    "payload.estimation.method_version": "regression_logistic_v1",
                    "payload.estimation.regression.method_version": "regression_logistic_v1",
                },
            }],
        },
        {
            "name": "extended_methods_publication_audit.json",
            "required_true": ["method_coverage.logistic"],
            "source_paths": ["validation/extended_methods_publication_audit.py"],
        },
    ],
    [{
        "name": "REGRESSION_LOGISTIC_V1.md",
        "required_phrases": [
            "regression_logistic_v1",
            "binary 0/1 values",
            "complete separation",
            "R `glm` comparison",
        ],
    }],
))
