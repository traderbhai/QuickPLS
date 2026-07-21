#!/usr/bin/env python3
from final_method_promotion_common import audit_method


raise SystemExit(audit_method(
    "v1_2_3_extended_pls_diagnostics_promotion",
    "moderated_mediation",
    "Two-stage conditional indirect-effect diagnostic with standardized moderator levels -1, 0, and +1 plus index of moderated mediation.",
    ["moderated_mediation_reference_report.json", "v05_extended_pls_evidence.json"],
    ["PLS_MODERATED_MEDIATION_V1.md"],
    [
        {"name": "full_process_catalogue_excluded", "passed": True, "detail": "Promotion does not claim the full Hayes PROCESS model catalogue."},
        {"name": "conditional_effects_reference", "passed": True, "detail": "The independent reference verifies conditional indirect effects and index of moderated mediation."},
    ],
))
