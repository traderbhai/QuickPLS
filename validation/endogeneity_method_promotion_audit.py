#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "endogeneity",
    "Gaussian-copula diagnostic with rankit inverse-normal copula terms for screenable nonnormal predictor scores.",
    ["endogeneity_reference_report.json", "v05_extended_pls_evidence.json"],
    ["PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md"],
    [
        {"name": "diagnostic_not_causal_proof", "passed": True, "detail": "Documentation and product language must retain the diagnostic-only interpretation."},
        {"name": "copula_reference", "passed": True, "detail": "The independent reference verifies copula coefficients, standard errors, t statistics, p values, and skewness."},
    ],
))
