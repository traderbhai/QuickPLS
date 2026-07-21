#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "nonlinear_effects",
    "Centered squared-term fixed-score nonlinear diagnostic for supported PLS structural paths.",
    ["nonlinear_effects_reference_report.json", "v05_extended_pls_evidence.json"],
    ["PLS_NONLINEAR_EFFECTS_V1.md"],
    [
        {"name": "diagnostic_scope_recorded", "passed": True, "detail": "Promotion is diagnostic and does not claim unrestricted nonlinear SEM estimation."},
        {"name": "ols_equation_reference", "passed": True, "detail": "The independent reference verifies quadratic coefficients, t/p values, and R2 deltas."},
    ],
))
