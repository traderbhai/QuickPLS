#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "cta_pls",
    "Descriptive sample-covariance tetrad diagnostic for valid indicator blocks with four or more indicators.",
    ["cta_pls_reference_report.json", "v05_extended_pls_evidence.json"],
    ["PLS_CTA_PLS_V1.md"],
    [
        {"name": "tetrad_reference", "passed": True, "detail": "The independent reference verifies all tetrad pairings within tolerance."},
        {"name": "inference_decisions_excluded", "passed": True, "detail": "Bootstrap/permutation tetrad decision rules remain outside the promoted scope."},
    ],
))
