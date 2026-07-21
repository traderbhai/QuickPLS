#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "cca",
    "Descriptive composite correlation residual diagnostic for recursive standardized PLS path models.",
    ["cca_reference_report.json", "v05_extended_pls_evidence.json"],
    ["PLS_CCA_V1.md"],
    [
        {"name": "decision_rules_excluded", "passed": True, "detail": "Bootstrap-based CCA decisions remain unsupported outside the promoted descriptive scope."},
        {"name": "invalid_settings_guard", "passed": True, "detail": "The reference artifact includes invalid PCA-setting guard evidence."},
    ],
))
